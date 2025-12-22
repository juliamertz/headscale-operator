use super::*;

impl Policy {
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

    fn render_configmap(&self, name: &str) -> Result<ConfigMap, Error> {
        let namespace = self.namespace().unwrap();
        let owner_ref = self.owner_ref(&()).unwrap();
        let config = PolicyConfig::from(self.spec.clone());

        Ok(ConfigMap::new(name)
            .namespace(&namespace)
            .labels(self.common_labels(name))
            .owner(owner_ref.clone())
            .data([("acl.json", serde_json::to_string(&config)?)]))
    }
}

#[kubus(event = Apply, finalizer = "headscale.juliamertz.dev/acl-policy-finalizer")]
async fn create_acl_policy(policy: Arc<Policy>, ctx: Arc<Context<State>>) -> Result<(), Error> {
    let client = ctx.client.clone();
    let namespace = policy.namespace().unwrap_or_default();

    let headscale = policy
        .spec
        .headscale_ref
        .resolve(client.clone(), &namespace)
        .await?;

    let name = headscale.acl_configmap_name();
    policy.render_configmap(&name)?.apply(&client).await?;

    Ok(())
}

#[kubus(event = Delete, finalizer = "headscale.juliamertz.dev/acl-policy-finalizer")]
async fn delete_acl_policy(policy: Arc<Policy>, ctx: Arc<Context<State>>) -> Result<(), Error> {
    let client = ctx.client.clone();
    let name = policy.name_any();
    let namespace = policy.namespace().unwrap();

    ConfigMap::new(name)
        .namespace(namespace)
        .delete(&client)
        .await?;

    Ok(())
}
