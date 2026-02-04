use k8s_openapi::NamespaceResourceScope;

use super::*;

fn expect_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|err| {
        panic!("expected environment variable {name} to be set, error: {err}")
    })
}

pub fn default_headscale_image() -> String {
    "ghcr.io/headscale/headscale:v0.28.0@sha256:51b1b9182bb6219e97374fa89af6b9320d6f87ecc739e328d5357ea4fa7a5ce3".to_string()
}
pub fn default_config_manager_image() -> String {
    expect_env("CONFIG_MANAGER_IMAGE")
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HeadscaleDeploymentOptions {
    #[serde(default = "default_headscale_image")]
    pub image: String,
    #[serde(default)]
    pub env: Vec<k8s_openapi_ext::corev1::EnvVar>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ConfigManagerOptions {
    #[serde(default = "default_config_manager_image")]
    pub image: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TLSOptions {
    pub existing_secret: Option<String>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "headscale.juliamertz.dev",
    version = "v1alpha1",
    kind = "Headscale",
    namespaced
)]
#[kube(status = "HeadscaleStatus")]
#[kube(
    printcolumn = r#"{"name": "Ready", "type": "boolean", "jsonPath": ".status.ready"}"#,
    printcolumn = r#"{"name": "Message", "type": "string", "jsonPath": ".status.message"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct HeadscaleSpec {
    #[schemars(schema_with = "preserve_unknown_fields")]
    pub config: serde_json::Value,
    pub deployment: HeadscaleDeploymentOptions,
    #[serde(default)]
    pub config_manager: ConfigManagerOptions,
    pub tls: TLSOptions,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HeadscaleStatus {
    pub ready: bool,
    pub message: Option<String>,
    pub last_updated: Option<String>,
}

#[derive(Default, Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct HeadscaleRef {
    pub name: String,
    pub namespace: Option<String>,
}

impl k8s_openapi::Resource for Headscale {
    const API_VERSION: &'static str = "headscale.juliamertz.dev/v1alpha1";
    const GROUP: &'static str = "headscale.juliamertz.dev";
    const KIND: &'static str = "Headscale";
    const VERSION: &'static str = "v1alpha1";
    const URL_PATH_SEGMENT: &'static str = "headscales";
    type Scope = NamespaceResourceScope;
}
