use crate::helper::IMAGES;

use super::*;

use anyhow::Context;
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi_ext::corev1::ResourceRequirements;

const ANNOTATION_INJECT_SIDECAR: &str = "headscale.juliamertz.dev/tailscale-inject-sidecar";
const ANNOTATION_EXTRA_ARGS: &str = "headscale.juliamertz.dev/tailscale-extra-args";
const ANNOTATION_IMAGE: &str = "headscale.juliamertz.dev/tailscale-image";
const ANNOTATION_AUTH_SECRET: &str = "headscale.juliamertz.dev/tailscale-auth-secret";
const ANNOTATION_RESOURCES: &str = "headscale.juliamertz.dev/tailscale-resources";

fn should_inject(req: &AdmissionRequest<DynamicObject>) -> bool {
    Pod::is(&req.kind)
        && req
            .get_annotation(ANNOTATION_INJECT_SIDECAR)
            .map(|v| v == "true")
            .unwrap_or(false)
}

fn build_sidecar_patch(
    extra_args: Option<&str>,
    image: Option<&str>,
    resources: Option<&str>,
    auth_secret: &str,
) -> Result<JsonPatch, Error> {
    let resources = resources
        .map(serde_json::from_str::<ResourceRequirements>)
        .transpose()
        .context("invalid resource requirements")?
        .map(serde_json::to_value)
        .unwrap_or_else(|| {
            Ok(serde_json::json!({
                "requests": { "cpu": "100m", "memory": "64Mi" },
            }))
        })?;

    let container = serde_json::json!({
        "name": "tailscale-sidecar",
        "image": image.unwrap_or(&IMAGES.tailscale),
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
        "resources": resources
    });

    Ok(JsonPatch(vec![PatchOperation::Add(AddOperation {
        path: "/spec/containers/-".parse()?,
        value: container,
    })]))
}

#[admission(mutating)]
pub async fn mutate(req: &AdmissionRequest<DynamicObject>) -> Result<AdmissionResponse, Error> {
    if should_inject(req) {
        let extra_args = req.get_annotation(ANNOTATION_EXTRA_ARGS);
        let image = req.get_annotation(ANNOTATION_IMAGE);
        let resources = req.get_annotation(ANNOTATION_RESOURCES);
        let Some(auth_secret) = req.get_annotation(ANNOTATION_AUTH_SECRET) else {
            let reason = format!("missing required '{ANNOTATION_AUTH_SECRET}' annotation");
            return Ok(AdmissionResponse::from(req).deny(reason));
        };

        let patch = build_sidecar_patch(extra_args, image, resources, auth_secret)?;

        Ok(AdmissionResponse::from(req).with_patch(patch)?)
    } else {
        Ok(AdmissionResponse::from(req))
    }
}
