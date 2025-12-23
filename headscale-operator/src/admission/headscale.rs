use super::*;

use crate::{crds::Headscale, handlers::default_config_manager_image};

#[admission(mutating)]
pub async fn mutate(req: &AdmissionRequest<DynamicObject>) -> Result<AdmissionResponse, Error> {
    let res = AdmissionResponse::from(req);
    if Headscale::is(&req.kind) {
        let object = &req.object.clone().expect("headscale resource object");
        let headscale: Headscale = parse_crd(object)?;

        if headscale.spec.config_manager.image.is_empty() {
            let patch = JsonPatch(vec![PatchOperation::Replace(ReplaceOperation {
                path: "/spec/configManager/image".parse()?,
                value: default_config_manager_image().into(),
            })]);
            return Ok(res.with_patch(patch)?);
        }
    }

    Ok(res)
}
