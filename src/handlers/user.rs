use crate::helper::CmdBuilder;

use super::*;

#[derive(Debug, Deserialize)]
pub struct UserData {
    pub id: u32,
}

impl User {
    async fn create(&self, client: &Client) -> Result<u32, Error> {
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

        let stdout = headscale.exec(&client, cmd).await?;
        let output: UserData = serde_json::from_str(&stdout)?;
        Ok(output.id)
    }

    async fn destroy(&self, client: &Client) -> Result<(), Error> {
        let Some(status) = self.status else {
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
        tracing::info!(
            { user = &name, id = &status.id },
            "user already exists, doing nothing"
        );
        return Ok(());
    }

    tracing::info!({ user = &name }, "creating user");

    let user_id = user.create(client).await?;
    let patch = json!({
       "status": {
            "id": user_id
        }
    });

    tracing::info!({ user = &name, id = &user_id }, "user created");

    let api = Api::<User>::namespaced(client.clone(), &namespace);
    api.patch_status(
        &name,
        &PatchParams::apply(env!("CARGO_PKG_NAME")),
        &Patch::Merge(&patch),
    )
    .await?;

    Ok(())
}

#[kubus(event = Delete, finalizer = "headscale.juliamertz.dev/user-finalizer")]
pub async fn destroy_user(user: Arc<User>, ctx: Arc<Context<State>>) -> Result<(), Error> {
    user.destroy(&ctx.client).await
}
