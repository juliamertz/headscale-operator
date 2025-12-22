use super::*;

use crate::handlers::HeadscaleRef;

use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Accept,
    Deny,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Acl {
    pub action: Action,
    pub src: Vec<String>,
    pub dst: Vec<String>,
}

pub type Group = String;
pub type Groups = BTreeMap<String, Vec<Group>>;

pub type TagOwner = String;
pub type TagOwners = BTreeMap<String, Vec<TagOwner>>;

pub type Host = String;
pub type Hosts = BTreeMap<String, Host>;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "headscale.juliamertz.dev",
    version = "v1alpha1",
    kind = "Policy",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct PolicySpec {
    pub headscale_ref: HeadscaleRef,
    pub groups: Option<Groups>,
    pub hosts: Option<Hosts>,
    pub tag_owners: Option<TagOwners>,
    pub acls: Vec<Acl>,
    // TODO:
    // pub auto_approvers: Option<AutoApproverPolicy>,
    // pub ssh: Option<Vec<Ssh>>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct PolicyConfig {
    pub groups: Option<Groups>,
    pub hosts: Option<Hosts>,
    pub tag_owners: Option<TagOwners>,
    pub acls: Vec<Acl>,
}

impl From<PolicySpec> for PolicyConfig {
    fn from(policy: PolicySpec) -> Self {
        Self {
            groups: policy.groups,
            hosts: policy.hosts,
            tag_owners: policy.tag_owners,
            acls: policy.acls,
        }
    }
}
