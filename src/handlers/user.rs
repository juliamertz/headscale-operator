use crate::helper::CmdBuilder;

use super::*;

impl UserRef {
    pub async fn resolve(&self, client: Client, namespace: impl ToString) -> kube::Result<User> {
        let namespace = self
            .namespace
            .clone()
            .unwrap_or_else(|| namespace.to_string());

        let api = Api::<User>::namespaced(client, &namespace);

        api.get(&self.name).await
    }
}

impl User {
    pub fn id(&self) -> Option<u32> {
        self.status.as_ref().map(|status| status.id)
    }

    async fn create(&self, client: &Client) -> Result<UserData, Error> {
        let name = self.name_any();
        let namespace = self.namespace_any();
        let headscale = self
            .spec
            .headscale_ref
            .resolve(client.clone(), &namespace)
            .await?;

        let cmd = CmdBuilder::default()
            .arg("users")
            .arg("create")
            .arg(&name)
            .option_arg("--display-name", self.spec.display_name.as_ref())
            .option_arg("--picture-url", self.spec.picture_url.as_ref())
            .option_arg("--email", self.spec.email.as_ref())
            .collect();

        let stdout = headscale.exec(client, cmd).await?;
        Ok(serde_json::from_str(&stdout)?)
    }

    async fn destroy(&self, client: &Client) -> Result<(), Error> {
        let Some(ref status) = self.status else {
            return Ok(());
        };

        let namespace = self.namespace_any();
        let headscale = self
            .spec
            .headscale_ref
            .resolve(client.clone(), &namespace)
            .await?;

        let cmd = CmdBuilder::default()
            .arg("users")
            .arg("destroy")
            .option_arg("--identifier", Some(status.id))
            .arg("--force")
            .collect();

        headscale.exec(client, cmd).await?;

        Ok(())
    }
}

#[kubus(event = Apply, finalizer = "headscale.juliamertz.dev/user-finalizer")]
pub async fn create_user(user: Arc<User>, ctx: Arc<Context<State>>) -> Result<(), Error> {
    let client = &ctx.client;
    let name = user.name_any();
    let namespace = user.namespace_any();

    // TODO: we should check if the user actually exists instead of assuming
    if let Some(ref status) = user.status {
        tracing::debug!(
            { user = &name, id = &status.id },
            "user already exists, doing nothing"
        );
        return Ok(());
    }

    tracing::info!({ user = &name }, "creating user");

    let data = user.create(client).await?;
    let status: UserStatus = data.into();

    tracing::info!({ user = &name, id = &status.id }, "user created");

    let api = Api::<User>::namespaced(client.clone(), &namespace);
    api.patch_status(
        &name,
        &PatchParams::apply(env!("CARGO_PKG_NAME")),
        &Patch::Merge(json!({ "status": status })),
    )
    .await?;

    Ok(())
}

#[kubus(event = Delete, finalizer = "headscale.juliamertz.dev/user-finalizer")]
pub async fn destroy_user(user: Arc<User>, ctx: Arc<Context<State>>) -> Result<(), Error> {
    user.destroy(&ctx.client).await
}
