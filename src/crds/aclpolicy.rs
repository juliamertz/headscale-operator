use super::*;

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum Action {
    Accept,
    Deny,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Policy {
    action: Action,
    src: Vec<String>,
    dst: Vec<String>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "kubus.io", version = "v1", kind = "ACLPolicy", namespaced)]
#[serde(rename_all = "camelCase")]
pub struct ACLPolicySpec {
    acls: Vec<Policy>,
}
