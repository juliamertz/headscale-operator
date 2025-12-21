use super::{Error, Result};

use std::error::Error as StdError;
use std::path::PathBuf;

use k8s_openapi::api::core::v1::ConfigMap;
use serde::Serialize;
use serde_json::Value;
use tokio::fs;
use tracing::{debug, error};

pub struct Config {
    pub acls: Value,
}

impl TryFrom<ConfigMap> for Config {
    type Error = Error;

    fn try_from(configmap: ConfigMap) -> Result<Self, Self::Error> {
        let data = configmap.data.unwrap_or_default();
        let content = data.get("acl.json").map(String::as_str).unwrap_or("{}");

        let acls = match serde_json::from_str(content) {
            Ok(value) => value,
            Err(err) => {
                error!(
                    { err = &err as &dyn StdError },
                    "failed to parse ACL policies as JSON"
                );
                return Err(Error::Json(err));
            }
        };

        Ok(Self { acls })
    }
}

pub struct ConfigManager {
    mount_path: PathBuf,
}

impl ConfigManager {
    const ACL_FILENAME: &str = "acl.json";

    pub fn new(mount_path: impl Into<PathBuf>) -> Self {
        let mount_path = mount_path.into();
        Self { mount_path }
    }

    pub async fn write<D: Serialize>(&self, data: &D) -> Result<()> {
        let path = self.mount_path.join(Self::ACL_FILENAME);
        let content = serde_json::to_vec(data)?;
        fs::write(&path, &content).await?;
        debug!("written {} bytes to {path:?}", content.len());
        Ok(())
    }
}
