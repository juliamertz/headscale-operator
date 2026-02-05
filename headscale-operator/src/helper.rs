use std::fmt::Debug;
use std::ops::Deref;

use async_trait::async_trait;
use k8s_openapi_ext::resource::Quantity;
use kube::api::{AttachParams, Execute};
use kube::{Api, Resource, ResourceExt as _};
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::io::AsyncReadExt;

#[derive(Debug, Default, Clone)]
pub struct CmdBuilder {
    buf: Vec<String>,
}

impl CmdBuilder {
    #[allow(dead_code)]
    pub fn new(bin: impl ToString) -> Self {
        let buf = vec![bin.to_string()];
        Self { buf }
    }

    pub fn arg(mut self, arg: impl ToString) -> Self {
        self.buf.push(arg.to_string());
        self
    }

    pub fn option_arg(self, name: impl ToString, arg: Option<impl ToString>) -> Self {
        if let Some(arg) = arg {
            self.arg(name).arg(arg)
        } else {
            self
        }
    }

    pub fn bool_arg(self, name: impl ToString, cond: bool) -> Self {
        if cond { self.arg(name) } else { self }
    }

    pub fn collect(self) -> Vec<String> {
        self.buf
    }
}

#[async_trait]
pub trait ResourceExt {
    fn namespace_any(&self) -> String;
}

impl<K> ResourceExt for K
where
    K: Resource,
{
    fn namespace_any(&self) -> String {
        self.namespace().unwrap_or_else(|| "default".to_string())
    }
}

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("failed to join attached process: {0}")]
    Kube(#[from] kube::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("non-success exit status: {0}, out: {1}")]
    Exit(i32, String),
    #[error("unknown-success exit status: {0}, out: {1}")]
    UnknownStatus(i32, String),
}

#[async_trait]
pub trait ExecuteExt {
    async fn exec_with_output<I, T>(&self, name: &str, command: I) -> Result<String, ExecError>
    where
        I: IntoIterator<Item = T> + Debug + Send + Sync + 'static,
        T: Into<String>;
}

#[async_trait]
impl<K> ExecuteExt for Api<K>
where
    K: Resource + Execute + Clone + DeserializeOwned + Send + Sync + 'static,
{
    async fn exec_with_output<I, T>(&self, name: &str, command: I) -> Result<String, ExecError>
    where
        I: IntoIterator<Item = T> + Debug + Send + Sync + 'static,
        T: Into<String>,
    {
        let attach_params = AttachParams::default()
            .container("headscale")
            .stdin(false)
            .stdout(true)
            .stderr(true);
        let mut process = self.exec(name, command, &attach_params).await?;

        let Some(output) = process
            .take_status()
            .expect("status has not been taken")
            .await
        else {
            todo!("no status retrieved");
        };

        let mut buf = String::new();
        match output.status.as_deref() {
            Some("Success") => {
                process.stdout().unwrap().read_to_string(&mut buf).await?;

                Ok(buf)
            }

            Some("Failure") => {
                process.stderr().unwrap().read_to_string(&mut buf).await?;

                Err(ExecError::Exit(output.code.unwrap_or_default(), buf))
            }

            _ => Err(ExecError::UnknownStatus(
                output.code.unwrap_or_default(),
                output
                    .message
                    .unwrap_or_else(|| "unknown kube response status".into()),
            )),
        }
    }
}

#[derive(Default)]
pub struct Resources(Vec<(String, Quantity)>);

impl Deref for Resources {
    type Target = [(String, Quantity)];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resources {
    pub fn cpu(mut self, quantity: impl ToString) -> Self {
        self.0
            .push(("cpu".to_string(), Quantity(quantity.to_string())));
        self
    }

    pub fn mem(mut self, quantity: impl ToString) -> Self {
        self.0
            .push(("memory".to_string(), Quantity(quantity.to_string())));
        self
    }

    pub fn inner(self) -> Vec<(String, Quantity)> {
        self.0
    }
}
