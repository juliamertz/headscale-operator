pub use kube::CustomResource;
pub use schemars::JsonSchema;
pub use serde::{Deserialize, Serialize};

pub mod aclpolicy;
pub mod headscale;

pub use aclpolicy::ACLPolicy;
pub use headscale::Headscale;

pub fn preserve_unknown_fields(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({ "x-kubernetes-preserve-unknown-fields": true })
}

