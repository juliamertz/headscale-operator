use crate::handlers::HeadscaleRef;

use super::*;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "headscale.juliamertz.dev",
    version = "v1alpha1",
    kind = "User",
    namespaced,
    status = "UserStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct UserSpec {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub picture_url: Option<String>,
    pub headscale_ref: HeadscaleRef,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserStatus {
    pub id: u32,
    pub name: String,
    pub created_at: Option<Timestamp>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub picture_url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserRef {
    pub name: String,
    pub namespace: Option<String>,
}

/// internal headscale data structure used for deserializing cli output
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UserData {
    pub id: u32,
    pub name: String,
    pub created_at: Option<Timestamp>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub picture_url: Option<String>,
}
