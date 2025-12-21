use kube::{
    api::{DynamicObject, GroupVersionKind},
    core::admission::AdmissionRequest,
};

pub mod sidecar;

pub trait AdmissionRequestExt {
    fn get_annotation<'a>(&'a self, name: impl AsRef<str>) -> Option<&'a str>;
}

impl AdmissionRequestExt for AdmissionRequest<DynamicObject> {
    fn get_annotation<'a>(&'a self, name: impl AsRef<str>) -> Option<&'a str> {
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
