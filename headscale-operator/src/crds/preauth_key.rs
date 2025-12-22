use super::*;

use crate::crds::user::UserRef;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "headscale.juliamertz.dev",
    version = "v1alpha1",
    kind = "PreauthKey",
    status = "PreauthKeyStatus",
    namespaced
)]
#[serde(default, rename_all = "camelCase")]
pub struct PreauthKeySpec {
    pub ephemeral: bool,
    pub reusable: bool,
    pub expiration: String,
    pub target_secret: Option<String>,
    pub user: UserRef,
}

impl Default for PreauthKeySpec {
    fn default() -> Self {
        Self {
            ephemeral: false,
            reusable: false,
            expiration: "1h".to_string(),
            target_secret: None,
            user: Default::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreauthKeyStatus {
    pub id: u32,
    pub user: crate::crds::user::UserStatus,
    pub reusable: bool,
    pub ephemeral: bool,
    pub expiration: Timestamp,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreauthKeyData {
    pub id: u32,
    pub user: crate::crds::user::UserData,
    pub key: String,
    #[serde(default)]
    pub reusable: bool,
    #[serde(default)]
    pub ephemeral: bool,
    pub expiration: Timestamp,
    pub created_at: Timestamp,
}
