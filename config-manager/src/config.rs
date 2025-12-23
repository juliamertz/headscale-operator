use super::{Error, Result};

use std::path::PathBuf;
use std::{error::Error as StdError, path::Path};

use k8s_openapi::api::core::v1::ConfigMap;
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

    async fn changed(&self, path: &Path, value: &Value) -> Result<bool> {
        if !path.exists() {
            return Ok(true);
        }

        let content = fs::read_to_string(path).await?;
        let current: Value = serde_json::from_str(&content)?;

        if value == &current {
            return Ok(false);
        }

        Ok(true)
    }

    pub async fn write(&self, path: &Path, value: &Value) -> Result<()> {
        let content = serde_json::to_vec(value)?;
        fs::write(&path, &content).await?;
        debug!("written {} bytes to {path:?}", content.len());
        Ok(())
    }

    pub async fn sync(&self, value: &Value) -> Result<bool> {
        let path = self.mount_path.join(Self::ACL_FILENAME);

        let changed = self.changed(&path, value).await.unwrap_or_else(|err| {
            error!(
                { err = &err as &dyn StdError },
                "failed to check for configuration changes"
            );
            true
        });

        if changed {
            self.write(&path, value).await?;
        }

        Ok(changed)
    }
}
