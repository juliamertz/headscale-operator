use super::*;

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct HeadscaleDeploymentOptions {
    pub env: Vec<k8s_openapi_ext::corev1::EnvVar>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TLSOptions {
    pub existing_secret: Option<String>,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "headscale.juliamertz.dev", version = "v1", kind = "Headscale", namespaced)]
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

