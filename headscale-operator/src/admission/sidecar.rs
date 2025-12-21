use k8s_openapi::{Resource, api::core::v1::Pod};
use kube::{
    api::{DynamicObject, GroupVersionKind},
    core::admission::{AdmissionRequest, AdmissionResponse},
};
use kubus::admission;

use crate::Error;

const ANNOTATION_INJECT_SIDECAR: &str = "headscale.juliamertz.dev/tailscale-inject-sidecar";
const ANNOTATION_EXTRA_ARGS: &str = "headscale.juliamertz.dev/tailscale-extra-args";
const ANNOTATION_IMAGE: &str = "headscale.juliamertz.dev/tailscale-image";
const ANNOTATION_AUTH_SECRET: &str = "headscale.juliamertz.dev/tailscale-auth-secret";

const DEFAULT_TAILSCALE_IMAGE: &str = "ghcr.io/tailscale/tailscale:v1.92.4";

pub trait ResourceGvkExt {
    fn is(kind: &GroupVersionKind) -> bool;
}

impl<K: Resource> ResourceGvkExt for K {
    fn is(kind: &GroupVersionKind) -> bool {
        kind.group == K::GROUP && kind.version == K::VERSION && kind.kind == K::KIND
    }
}

fn get_annotation<'a>(req: &'a AdmissionRequest<DynamicObject>, name: &str) -> Option<&'a str> {
    req.object
        .as_ref()
        .and_then(|obj| obj.metadata.annotations.as_ref())
        .and_then(|ann| ann.get(name).map(|v| v.as_str()))
}

fn should_inject_sidecar(req: &AdmissionRequest<DynamicObject>) -> bool {
    Pod::is(&req.kind)
        && get_annotation(req, ANNOTATION_INJECT_SIDECAR)
            .map(|v| v == "true")
            .unwrap_or(false)
}

#[admission(mutating)]
pub async fn mutate(req: &AdmissionRequest<DynamicObject>) -> Result<AdmissionResponse, Error> {
    if should_inject_sidecar(req) {
        let extra_args = get_annotation(req, ANNOTATION_EXTRA_ARGS);
        let image = get_annotation(req, ANNOTATION_IMAGE);
        let Some(auth_secret) = get_annotation(req, ANNOTATION_AUTH_SECRET) else {
            let reason = format!("missing required '{ANNOTATION_AUTH_SECRET}' annotation");
            return Ok(AdmissionResponse::from(req).deny(reason));
        };

        let patch = build_sidecar_patch(extra_args, image, auth_secret)?;

        Ok(AdmissionResponse::from(req).with_patch(patch).unwrap())
    } else {
        Ok(AdmissionResponse::from(req))
    }
}

fn build_sidecar_patch(
    extra_args: Option<&str>,
    image: Option<&str>,
    auth_secret: &str,
) -> Result<json_patch::Patch, Error> {
    let container = serde_json::json!({
        "name": "tailscale-sidecar",
        "image": image.unwrap_or(DEFAULT_TAILSCALE_IMAGE),
        "securityContext": {
            "capabilities": {
                "add": ["NET_ADMIN"]
            }
        },
        "env": [
            { "name": "TS_EXTRA_ARGS", "value": extra_args.unwrap_or_default() },
            { "name": "TS_USERSPACE", "value": "false" },
            { "name": "TS_ACCEPT_DNS", "value": "true" },
            { "name": "TS_KUBE_SECRET", "value": "" },
            { "name": "TS_DEBUG_FIREWALL_MODE", "value": "nftables" },
            {
                "name": "TS_AUTHKEY",
                "valueFrom": {
                    "secretKeyRef": {
                        "name": auth_secret,
                        "key": "authkey"
                    }
                }
            },
            {
                "name": "POD_NAME",
                "valueFrom": {
                    "fieldRef": {
                        "fieldPath": "metadata.name",
                    }
                }
            },
            {
                "name": "POD_UID",
                "valueFrom": {
                    "fieldRef": {
                        "fieldPath": "metadata.uid",
                    }
                }
            },
        ],
        "resources": {
            "requests": { "cpu": "100m", "memory": "64Mi" },
            "limits": { "cpu": "200m", "memory": "128Mi" }
        }
    });

    let ops = vec![json_patch::PatchOperation::Add(json_patch::AddOperation {
        path: "/spec/containers/-".parse().unwrap(),
        value: container,
    })];

    Ok(json_patch::Patch(ops))
}
