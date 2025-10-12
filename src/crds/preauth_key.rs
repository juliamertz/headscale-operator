use super::*;

#[derive(Default, Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct HeadscaleRef {
    name: String,
    namespace: Option<String>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "kubus.io", version = "v1", kind = "PreauthKey", namespaced)]
#[serde(default, rename_all = "camelCase")]
pub struct PreauthKeySpec {
    pub ephemeral: bool,
    pub reusable: bool,
    pub expiration: String,
    pub user_id: Option<u32>,
    pub target_secret: Option<String>,
    pub headscale_ref: HeadscaleRef,
}

impl Default for PreauthKeySpec {
    fn default() -> Self {
        Self {
            ephemeral: false,
            reusable: false,
            expiration: "1h".to_string(),
            user_id: None,
            target_secret: None,
            headscale_ref: Default::default(),
        }
    }
}
