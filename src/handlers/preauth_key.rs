use crate::ext::PodOwner;
use anyhow::anyhow;
use kube::api::AttachParams;
use tokio::io::AsyncReadExt;

use super::*;

impl HeadscaleRef {
    async fn resolve(&self, client: Client, namespace: impl ToString) -> kube::Result<Headscale> {
        let namespace = self
            .namespace
            .clone()
            .unwrap_or_else(|| namespace.to_string());

        let api = Api::<Headscale>::namespaced(client, &namespace);

        api.get(&self.name).await
    }
}

impl PreauthKey {
    fn common_labels(&self, name: impl ToString) -> impl Iterator<Item = (&'static str, String)> {
        let name = name.to_string();
        let manager = env!("CARGO_PKG_NAME").to_string();
        let version = env!("CARGO_PKG_VERSION").to_string();
        let instance = format!("headscale-{name}");
        let part_of = "headscale".to_string();
        [
            ("app.kubernetes.io/name", name),
            ("app.kubernetes.io/managed-by", manager),
            ("app.kubernetes.io/instance", instance),
            ("app.kubernetes.io/version", version),
            ("app.kubernetes.io/part-of", part_of),
        ]
        .into_iter()
    }

    async fn generate_key(&self, client: Client) -> Result<String, Error> {
        let namespace = self.namespace().unwrap();
        let headscale_ref = self
            .spec
            .headscale_ref
            .as_ref()
            .context("missing field headscaleRef")?;

        let stateful_set_name = headscale_ref
            .resolve(client.clone(), &namespace)
            .await?
            .stateful_set_name();

        let api: Api<StatefulSet> = Api::namespaced(client.clone(), &namespace);
        let stateful_set = api.get(&stateful_set_name).await?;

        let first_pod = stateful_set.get_pod(client.clone()).await?.unwrap();

        let user_id = self
            .spec
            .user_id
            .context("user id field must be set")?
            .to_string();

        let mut cmd: Vec<String> = ["headscale", "preauthkeys", "create", "--user", &user_id]
            .into_iter()
            .map(Into::into)
            .collect();

        if self.spec.ephemeral {
            cmd.push("--ephemeral".into());
        }
        if self.spec.reusable {
            cmd.push("--reusable".into());
        }

        let api: Api<Pod> = Api::namespaced(client.clone(), &namespace);
        let pod_name = first_pod.name_unchecked();
        let (status, mut stdout, mut stderr) = api.exec_with_output(&pod_name, cmd).await?;

        if status.status.unwrap_or_default().as_str() == "Failure" {
            panic!(
                "{} : {}",
                status.message.unwrap_or_default(),
                stderr.read_to_string().await.unwrap()
            );
        }

        let authkey = stdout.read_to_string().await.unwrap().trim().to_string();

        if authkey.is_empty() {
            unreachable!() // TODO:
        }

        Ok(authkey)
    }

    async fn revoke_key(&self, client: Client, key: &str) -> Result<(), Error> {
        let namespace = self.namespace().unwrap();
        let headscale_ref = self
            .spec
            .headscale_ref
            .as_ref()
            .context("missing field headscaleRef")?;

        let stateful_set_name = headscale_ref
            .resolve(client.clone(), &namespace)
            .await?
            .stateful_set_name();

        let api: Api<StatefulSet> = Api::namespaced(client.clone(), &namespace);
        let stateful_set = api.get(&stateful_set_name).await?;

        let first_pod = stateful_set.get_pod(client.clone()).await?.unwrap();
        let user_id = self
            .spec
            .user_id
            .context("user id field must be set")?
            .to_string();

        let cmd: Vec<String> = [
            "headscale",
            "preauthkeys",
            "revoke",
            "--user",
            &user_id,
            key,
        ]
        .into_iter()
        .map(Into::into)
        .collect();

        let api = Api::<Pod>::namespaced(client.clone(), &namespace);
        let pod_name = first_pod.name_unchecked();

        let mut proc = api.exec(&pod_name, cmd, &AttachParams::default()).await?;
        let handle = proc.take_status().unwrap().await.unwrap();
        let status = handle.status.unwrap_or_else(|| "Unknown".into());

        if &status != "Success" {
            let mut stderr = String::new();
            proc.stderr()
                .unwrap()
                .read_to_string(&mut stderr)
                .await
                .unwrap();

            return Err(anyhow!("non success exit status: {:?} output: {}", status, stderr).into());
        }

        Ok(())
    }

    fn secret_name(&self) -> String {
        let name = self.name_unchecked();
        self.spec
            .target_secret
            .clone()
            .unwrap_or_else(|| format!("headscale-preauth-{name}"))
    }

    fn render_secret(&self, preauth_key: String) -> Secret {
        let namespace = self.namespace().unwrap_or_default();
        let owner_ref = self.owner_ref(&()).unwrap_or_default();
        let secret_name = self.secret_name();

        Secret::new(&secret_name)
            .namespace(&namespace)
            .labels(self.common_labels(&secret_name))
            .owner(owner_ref)
            .string_data([("authkey", preauth_key)])
    }
}

#[kubus(event = Apply, finalizer = "kubus.io/preauth-key-finalizer")]
async fn create_preauth_key(
    resource: Arc<PreauthKey>,
    ctx: Arc<Context<State>>,
) -> Result<(), Error> {
    let client = ctx.client.clone();

    let namespace = resource.namespace().unwrap();
    let secret_name = resource.secret_name();

    let exists = Secret::new(&secret_name)
        .namespace(&namespace)
        .exists(&client)
        .await?;

    if !exists {
        let preauth_key = resource.generate_key(client.clone()).await?;
        let secret = resource.render_secret(preauth_key).apply(&client).await?;
        secret.apply(&client).await?;
    }

    Ok(())
}

#[kubus(event = Delete, finalizer = "kubus.io/preauth-key-finalizer")]
async fn revoke_preauth_key(
    resource: Arc<PreauthKey>,
    ctx: Arc<Context<State>>,
) -> Result<(), Error> {
    let client = ctx.client.clone();

    let name = resource.name_unchecked();
    let namespace = resource.namespace().unwrap_or_default();
    let secret_name = resource.secret_name();

    let api = Api::<Secret>::namespaced(client.clone(), &namespace);
    let secret = api.get(&secret_name).await?;

    let data = secret
        .data
        .clone()
        .context("expected preauth key secret data")?;

    let preauth_key = data
        .get("authkey")
        .context("unable to get preauth key secret value")
        .map(|v| v.0.clone())
        .map(String::from_utf8)
        .unwrap()
        .unwrap();

    resource.revoke_key(client.clone(), &preauth_key).await?;

    Secret::new(secret_name)
        .namespace(namespace)
        .delete(&client)
        .await?;

    Ok(())
}
