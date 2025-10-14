use std::fmt::Debug;

use async_trait::async_trait;
use k8s_openapi_ext::appsv1::StatefulSet;
use k8s_openapi_ext::corev1::Pod;
use kube::api::{AttachParams, Execute, ListParams};
use kube::core::Selector;
use kube::{Api, Client, Resource, ResourceExt};
use serde::de::DeserializeOwned;
use tokio::io::AsyncReadExt;

#[async_trait]
pub trait ExecuteExt {
    async fn exec_with_output<I, T>(&self, name: &str, command: I) -> Result<String, String>
    where
        I: IntoIterator<Item = T> + Debug + Send + Sync + 'static,
        T: Into<String>;
}

#[async_trait]
impl<K> ExecuteExt for Api<K>
where
    K: Resource + Execute + Clone + DeserializeOwned + Send + Sync + 'static,
{
    async fn exec_with_output<I, T>(&self, name: &str, command: I) -> Result<String, String>
    where
        I: IntoIterator<Item = T> + Debug + Send + Sync + 'static,
        T: Into<String>,
    {
        let attach_params = AttachParams::default()
            .stdin(false)
            .stdout(true)
            .stderr(true);
        let mut process = self.exec(name, command, &attach_params).await.unwrap();

        let output = process.take_status().unwrap().await.unwrap();

        let mut buf = String::new();
        match output.status.as_deref() {
            Some("Success") => {
                process
                    .stdout()
                    .unwrap()
                    .read_to_string(&mut buf)
                    .await
                    .map_err(|err| format!("unable to read stdout: {err}"))?;

                Ok(buf)
            }

            Some("Failure") => {
                process
                    .stderr()
                    .unwrap()
                    .read_to_string(&mut buf)
                    .await
                    .map_err(|err| format!("unable to read stdout: {err}"))?;

                Err(buf)
            }

            _ => Err("unknown kube response status".to_string()),
        }
    }
}

#[async_trait]
pub trait PodOwner {
    async fn get_pod(&self, client: Client) -> kube::Result<Option<Pod>>;
}

#[async_trait]
impl PodOwner for StatefulSet {
    async fn get_pod(&self, client: Client) -> kube::Result<Option<Pod>> {
        let namespace = self.namespace().unwrap_or_default();
        let spec = self.spec.clone().unwrap_or_default();

        let selector = Selector::from_iter(spec.selector.match_labels.unwrap());
        let list_params = ListParams::default().labels_from(&selector);

        let api = Api::<Pod>::namespaced(client.clone(), &namespace);
        let pods = api.list(&list_params).await?;

        Ok(pods.items.first().cloned())
    }
}
