use std::fmt::Debug;

use async_trait::async_trait;
use k8s_openapi_ext::appsv1::{Deployment, StatefulSet};
use k8s_openapi_ext::corev1::Pod;
use k8s_openapi_ext::metav1::Status;
use kube::api::{AttachParams, Execute, ListParams};
use kube::core::Selector;
use kube::{Api, Client, Resource, ResourceExt};
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct ExecuteOutput<T>
where
    T: AsyncRead + AsyncReadExt + Unpin,
{
    reader: T,
}

impl<T> ExecuteOutput<T>
where
    T: AsyncRead + AsyncReadExt + Unpin,
{
    pub fn new(reader: T) -> Self {
        Self { reader }
    }
}

impl<T> ExecuteOutput<T>
where
    T: AsyncRead + AsyncReadExt + Unpin,
{
    pub async fn read_to_string(&mut self) -> Option<String> {
        let mut out = String::new();
        self.reader.read_to_string(&mut out).await.ok()?;
        Some(out)
    }
}

pub type Stdout<T> = ExecuteOutput<T>;
pub type Stderr<T> = ExecuteOutput<T>;

#[async_trait]
pub trait ExecuteExt {
    async fn exec_with_output<I, T>(
        &self,
        name: &str,
        command: I,
    ) -> kube::Result<(
        Status,
        Stdout<impl AsyncRead + Unpin>,
        Stderr<impl AsyncRead + Unpin>,
    )>
    where
        I: IntoIterator<Item = T> + Debug + Send + Sync + 'static,
        T: Into<String>;
}

#[async_trait]
impl<K> ExecuteExt for Api<K>
where
    K: Resource + Execute + Clone + DeserializeOwned + Send + Sync + 'static,
{
    async fn exec_with_output<I, T>(
        &self,
        name: &str,
        command: I,
    ) -> kube::Result<(
        Status,
        Stdout<impl AsyncRead + Unpin>,
        Stderr<impl AsyncRead + Unpin>,
    )>
    where
        I: IntoIterator<Item = T> + Debug + Send + Sync + 'static,
        T: Into<String>,
    {
        let attach_params = AttachParams::default()
            .stdin(false)
            .stdout(true)
            .stderr(true);
        let mut process = self.exec(name, command, &attach_params).await?;

        let stdout = Stdout::new(process.stdout().unwrap());
        let stderr = Stderr::new(process.stderr().unwrap());
        let status = process.take_status().unwrap().await.unwrap();

        Ok((status, stdout, stderr))
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
