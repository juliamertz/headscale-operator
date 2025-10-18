use clap::{Parser, Subcommand};
use kube::{Client, CustomResourceExt};
use kubus::{Operator, print_crds};
use std::fmt::Debug;
use std::io::Write;
use thiserror::Error;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub(crate) mod crds;
pub(crate) mod handlers;
pub(crate) mod helper;

use crds::*;

use crate::handlers::User;
use crate::handlers::aclpolicy::{create_acl_policy, delete_acl_policy};
use crate::handlers::headscale::{cleanup_headscale, deploy_headscale};
use crate::handlers::preauth_key::{create_preauth_key, revoke_preauth_key};
use crate::handlers::user::{create_user, destroy_user};

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
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .unwrap();

    match opts.command.unwrap_or_default() {
        Command::Crd => print_crds![Headscale, ACLPolicy, PreauthKey, User],

        Command::Run => {
            let client = Client::try_default().await.unwrap();
            let state = State {};

            Operator::builder()
                .with_context((client, state))
                .handler(create_user)
                .handler(destroy_user)
                .handler(deploy_headscale)
                .handler(cleanup_headscale)
                .handler(create_acl_policy)
                .handler(delete_acl_policy)
                .handler(create_preauth_key)
                .handler(revoke_preauth_key)
                .run()
                .await?
        }
    };

    Ok(())
}
