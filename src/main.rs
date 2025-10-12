use clap::{Parser, Subcommand};
use k8s_openapi::List;
use kube::{Client, CustomResourceExt};
use kubus::Operator;
use std::fmt::Debug;
use std::io::Write;
use thiserror::Error;

pub(crate) mod crds;
pub(crate) mod ext;
pub(crate) mod handlers;

use crds::*;

use crate::handlers::aclpolicy::{create_acl_policy, delete_acl_policy};
use crate::handlers::headscale::{cleanup_headscale, deploy_headscale};
use crate::handlers::preauth_key::create_preauth_key;

#[derive(Debug, Error)]
enum Error {
    #[error("kube error: {0}")]
    Kube(#[from] kube::Error),
    #[error("kubus error: {0}")]
    Kubus(#[from] kubus::Error),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("anyhow: {0}")]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Parser)]
#[command(name = "headscale-operator")]
#[command(about = "Kubernetes operator for the Headscale VPN")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Default)]
enum Command {
    Crd,
    #[default]
    Run,
}

#[derive(Debug, Clone)]
pub struct State {}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = Cli::parse();
    tracing_subscriber::fmt().init();

    match opts.command.unwrap_or_default() {
        Command::Crd => {
            let mut list = List::default();
            list.items = vec![Headscale::crd(), ACLPolicy::crd(), PreauthKey::crd()];
            let yaml = serde_yaml::to_string(&list).unwrap();
            let mut stdout = std::io::stdout();
            stdout.write_all(yaml.as_bytes())?;
        }
        Command::Run => {
            let client = Client::try_default().await.unwrap();
            let state = State {};

            Operator::builder()
                .with_context((client, state))
                .handler(deploy_headscale)
                .handler(cleanup_headscale)
                .handler(create_acl_policy)
                .handler(delete_acl_policy)
                .handler(create_preauth_key)
                .run()
                .await?
        }
    };

    Ok(())
}
