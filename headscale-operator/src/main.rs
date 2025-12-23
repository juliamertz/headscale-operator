use clap::{Parser, Subcommand};
use kube::{Client, CustomResourceExt};
use kubus::{Operator, print_crds};
use std::fmt::Debug;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub(crate) mod admission;
pub(crate) mod crds;
pub(crate) mod handlers;
pub(crate) mod helper;
pub(crate) mod rbac;

use crds::*;

use crate::handlers::User;
use crate::handlers::headscale::{cleanup_headscale, deploy_headscale};
use crate::handlers::policy::{create_acl_policy, delete_acl_policy};
use crate::handlers::preauth_key::{create_preauth_key, revoke_preauth_key};
use crate::handlers::user::{create_user, destroy_user};

#[derive(Debug, Error)]
pub enum Error {
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
    #[error("failed to Seserialize patch: {0}")]
    SerializePatch(#[from] kube::core::admission::SerializePatchError),
    #[error("invalid json pointer: {0}")]
    JsonPtr(#[from] json_patch::jsonptr::ParseError),
}

#[derive(Parser)]
#[command(name = "headscale-operator")]
#[command(about = "Kubernetes operator for the Headscale VPN")]
struct Opts {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Crd,
    Run {
        #[arg(long, env = "TLS_CERT_PATH")]
        tls_path: Option<PathBuf>,

        #[arg(env = "CONFIG_MANAGER_IMAGE")]
        config_manager_image: String,
    },
}

pub type State = ();

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = Opts::parse();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .unwrap();

    match opts.command {
        Command::Crd => print_crds![Headscale, Policy, PreauthKey, User],

        Command::Run { tls_path, .. } => {
            let client = Client::try_default().await.unwrap();
            let mut operator = Operator::builder()
                .with_context((client, ()))
                .handler(create_user)
                .handler(destroy_user)
                .handler(deploy_headscale)
                .handler(cleanup_headscale)
                .handler(create_acl_policy)
                .handler(delete_acl_policy)
                .handler(create_preauth_key)
                .handler(revoke_preauth_key)
                .mutator(admission::headscale::mutate)
                .mutator(admission::sidecar::mutate);

            if let Some(tls_path) = tls_path {
                operator = operator.with_tls_certs(tls_path)
            }

            operator.run().await?
        }
    };

    Ok(())
}
