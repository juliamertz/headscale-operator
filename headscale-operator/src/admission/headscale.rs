use super::*;

use crate::{crds::Headscale, handlers::default_config_manager_image};

#[admission(mutating)]
pub async fn mutate(req: &AdmissionRequest<DynamicObject>) -> Result<AdmissionResponse, Error> {
    let res = AdmissionResponse::from(req);
    if Headscale::is(&req.kind) {
        let patch = JsonPatch(vec![
            PatchOperation::Test(TestOperation {
                path: "/spec/configManager/image".parse()?,
                value: "".into(),
            }),
            PatchOperation::Replace(ReplaceOperation {
                path: "/spec/configManager/image".parse()?,
                value: default_config_manager_image().into(),
            }),
        ]);
        Ok(res.with_patch(patch)?)
    } else {
        Ok(res)
    }
}
