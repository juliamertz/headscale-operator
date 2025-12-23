pub use crate::Error;
pub use json_patch::Patch as JsonPatch;
pub use json_patch::{AddOperation, PatchOperation, ReplaceOperation};
pub use kube::api::{DynamicObject, GroupVersionKind};
pub use kube::core::admission::{AdmissionRequest, AdmissionResponse};
pub use kubus::admission;
use serde::de::DeserializeOwned;

pub mod headscale;
pub mod sidecar;

pub trait AdmissionRequestExt {
    fn get_annotation(&self, name: impl AsRef<str>) -> Option<&str>;
}

impl AdmissionRequestExt for AdmissionRequest<DynamicObject> {
    fn get_annotation(&self, name: impl AsRef<str>) -> Option<&str> {
        self.object
            .as_ref()
            .and_then(|obj| obj.metadata.annotations.as_ref())
            .and_then(|ann| ann.get(name.as_ref()).map(|v| v.as_str()))
    }
}

pub trait ResourceGvkExt {
    fn is(kind: &GroupVersionKind) -> bool;
}

impl<K: k8s_openapi::Resource> ResourceGvkExt for K {
    fn is(kind: &GroupVersionKind) -> bool {
        kind.group == K::GROUP && kind.version == K::VERSION && kind.kind == K::KIND
    }
}

fn parse_crd<T: DeserializeOwned>(obj: &DynamicObject) -> Result<T, serde_json::Error> {
    let value = serde_json::to_value(obj)?;
    serde_json::from_value(value)
}
