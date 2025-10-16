use crate::handlers::HeadscaleRef;

use super::*;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "headscale.juliamertz.dev",
    version = "v1",
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

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserStatus {
    pub id: u32,
}
