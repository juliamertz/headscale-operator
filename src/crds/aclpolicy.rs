use super::*;

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Accept,
    Deny,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Rule {
    pub action: Action,
    pub src: Vec<String>,
    pub dst: Vec<String>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "headscale.juliamertz.dev", version = "v1", kind = "ACLPolicy", namespaced)]
#[serde(rename_all = "camelCase")]
pub struct ACLPolicySpec {
    pub rules: Vec<Rule>,
}
