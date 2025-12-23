pub use kube::CustomResource;
pub use schemars::JsonSchema;
pub use serde::{Deserialize, Serialize};
pub use serde_with::skip_serializing_none;

pub mod headscale;
pub mod policy;
pub mod preauth_key;
pub mod user;

pub use headscale::Headscale;
pub use policy::Policy;
pub use preauth_key::PreauthKey;

/// serialized timestamp format that headscale uses
#[derive(Debug, Default, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Timestamp {
    seconds: u64,
    nanos: u64,
}

pub fn preserve_unknown_fields(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({ "x-kubernetes-preserve-unknown-fields": true })
}
