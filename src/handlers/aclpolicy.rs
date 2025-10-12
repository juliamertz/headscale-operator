use std::sync::Arc;

use k8s_openapi_ext::corev1::ConfigMap;
use k8s_openapi_ext::*;
use kube::{Resource, ResourceExt};
use kubus::{ApiExt, Context, kubus};
use serde_json::json;

use crate::{Error, State, crds::ACLPolicy};

impl ACLPolicy {
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

    fn render_configmap(&self) -> ConfigMap {
        let name = self.name_unchecked();
        let namespace = self.namespace().unwrap();
        let owner_ref = self.owner_ref(&()).unwrap();

        let rules = serde_json::to_value(&self.spec.rules).unwrap();

        let name = format!("headscale-acl-{name}");
        ConfigMap::new(&name)
            .namespace(&namespace)
            .labels(self.common_labels(&name))
            .owner(owner_ref.clone())
            .data([("acl.json", json!({ "acls": rules }))])
    }
}

#[kubus(event = Apply, finalizer = "kubus.io/acl-policy-finalizer")]
async fn create_acl_policy(policy: Arc<ACLPolicy>, ctx: Arc<Context<State>>) -> Result<(), Error> {
    let client = ctx.client.clone();

    policy.render_configmap().apply(&client).await?;

    Ok(())
}

#[kubus(event = Delete, finalizer = "kubus.io/acl-policy-finalizer")]
async fn delete_acl_policy(policy: Arc<ACLPolicy>, ctx: Arc<Context<State>>) -> Result<(), Error> {
    let client = ctx.client.clone();

    policy.render_configmap().delete(&client).await?;

    Ok(())
}
