use crate::helper::CmdBuilder;

use super::*;

impl From<UserData> for UserStatus {
    fn from(data: UserData) -> Self {
        UserStatus {
            id: data.id,
            name: data.name,
            created_at: data.created_at,
            email: data.email,
            display_name: data.display_name,
            picture_url: data.picture_url,
        }
    }
}

impl From<PreauthKeyData> for PreauthKeyStatus {
    fn from(data: PreauthKeyData) -> Self {
        PreauthKeyStatus {
            id: data.id,
            user: data.user.into(),
            reusable: data.reusable,
            ephemeral: data.ephemeral,
            expiration: data.expiration,
            created_at: data.created_at,
        }
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

    async fn generate_key(&self, client: Client) -> Result<PreauthKeyData, Error> {
        let namespace = self.namespace_any();
        let user = self.spec.user.resolve(client.clone(), &namespace).await?;
        let user_id = user.id().context("user is missing an id")?;

        let cmd = CmdBuilder::default()
            .arg("preauthkeys")
            .arg("create")
            .option_arg("--user", Some(user_id))
            .option_arg("--expiration", Some(&self.spec.expiration))
            .bool_arg("--ephemeral", self.spec.ephemeral)
            .bool_arg("--reusable", self.spec.reusable)
            .collect();

        let headscale = user
            .spec
            .headscale_ref
            .resolve(client.clone(), &user.namespace_any())
            .await?;

        let stdout = headscale.exec(&client, cmd).await?;
        let authkey = serde_json::from_str(stdout.trim())?;
        Ok(authkey)
    }

    async fn revoke_key(&self, client: Client, key: &str) -> Result<(), Error> {
        let namespace = self.namespace().unwrap();
        let user = self.spec.user.resolve(client.clone(), &namespace).await?;
        let user_id = user.id().context("user is missing an id")?;

        let headscale = user
            .spec
            .headscale_ref
            .resolve(client.clone(), &user.namespace_any())
            .await?;

        let api: Api<StatefulSet> = Api::namespaced(client.clone(), &namespace);
        let stateful_set = api.get(&headscale.stateful_set_name()).await?;
        let first_pod = stateful_set.get_pod(client.clone()).await?.unwrap();

        let cmd: Vec<String> = [
            "headscale",
            "preauthkeys",
            "revoke",
            "--user",
            &user_id.to_string(),
            key,
        ]
        .into_iter()
        .map(Into::into)
        .collect();

        let api = Api::<Pod>::namespaced(client.clone(), &namespace);
        let pod_name = first_pod.name_unchecked();

        api.exec_with_output(&pod_name, cmd)
            .await
            .map_err(|stderr| anyhow!("error revoking preauth key: {stderr}"))?;

        Ok(())
    }

    fn secret_name(&self) -> String {
        let name = self.name_unchecked();
        self.spec
            .target_secret
            .clone()
            .unwrap_or_else(|| format!("headscale-preauth-{name}"))
    }

    fn render_secret(&self, preauth_key: impl ToString) -> Secret {
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

#[kubus(event = Apply, finalizer = "headscale.juliamertz.dev/preauth-key-finalizer")]
async fn create_preauth_key(
    resource: Arc<PreauthKey>,
    ctx: Arc<Context<State>>,
) -> Result<(), Error> {
    let client = ctx.client.clone();

    let name = resource.name_any();
    let namespace = resource.namespace_any();
    let secret_name = resource.secret_name();

    let exists = Secret::new(&secret_name)
        .namespace(&namespace)
        .exists(&client)
        .await?;

    if !exists {
        let data = resource.generate_key(client.clone()).await?;

        let secret = resource.render_secret(&data.key);
        secret.apply(&client).await?;

        let status: PreauthKeyStatus = data.into();
        let api = Api::<PreauthKey>::namespaced(client.clone(), &namespace);
        api.patch_status(
            &name,
            &PatchParams::default(),
            &Patch::Merge(json!({ "status": status })),
        )
        .await?;
        tracing::info!("succesfully patched status");
    }

    Ok(())
}

#[kubus(event = Delete, finalizer = "headscale.juliamertz.dev/preauth-key-finalizer")]
async fn revoke_preauth_key(
    resource: Arc<PreauthKey>,
    ctx: Arc<Context<State>>,
) -> Result<(), Error> {
    let client = ctx.client.clone();

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
