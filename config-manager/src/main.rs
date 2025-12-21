mod config;
mod process;

use std::io;
use std::path::PathBuf;
use std::{error::Error as StdError, time::Duration};

use clap::{Parser, Subcommand};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::runtime::WatchStreamExt;
use kube::{Api, Client};
use tokio::time::sleep;
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{Config, ConfigManager};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("kube error: {0}")]
    Kube(#[from] kube::error::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("process error: {0}")]
    Process(#[from] process::Error),
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Parser)]
#[command(name = "config-manager")]
struct Opts {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long, env = "CONFIGMAP_NAME")]
    configmap_name: String,

    #[arg(long, env = "MOUNT_PATH")]
    mount_path: PathBuf,
}

#[derive(Subcommand, Default)]
enum Command {
    Init,
    #[default]
    Run,
}

struct Context {
    opts: Opts,
    manager: ConfigManager,
    api: Api<ConfigMap>,
}

fn find_headscale_proc() -> io::Result<process::Process> {
    let mut processes = process::ProcessIter::try_new()?;
    let process = processes
        .find(|process| {
            process
                .cmdline
                .as_ref()
                .map(|cmdline| cmdline.starts_with("headscale\0serve\0"))
                .unwrap_or_default()
        })
        .ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "unable to find pid for headscale",
        ));

    if let Ok(ref process) = process {
        debug!({ pid = process.pid, cmd = process.cmdline }, "found headscale process");
    }

    process
}

async fn init(ctx: Context) -> Result<()> {
    let configmap = ctx.api.get(&ctx.opts.configmap_name).await?;
    let config = Config::try_from(configmap)?;

    ctx.manager.write(&config.acls).await?;
    Ok(())
}

async fn handle_event(ctx: &Context, configmap: ConfigMap) -> Result<()> {
    let config = Config::try_from(configmap)?;
    ctx.manager.write(&config.acls).await?;

    let headscale_process = find_headscale_proc()?;
    headscale_process.sighup()?;

    info!("sent SIGHUP to headscale container");
    Ok(())
}

async fn run(ctx: Context) -> Result<()> {
    info!("starting headscale ACL manager");

    let watcher_config = kube::runtime::watcher::Config {
        field_selector: Some(format!("metadata.name={}", &ctx.opts.configmap_name)),
        page_size: Some(10),
        ..Default::default()
    };
    let watcher = kube::runtime::watcher(ctx.api.clone(), watcher_config);
    let mut stream = watcher.applied_objects().boxed();

    loop {
        let event = match stream.try_next().await {
            Ok(event) => event,
            Err(err) => {
                error!({ err = &err as &dyn StdError }, "unable to read event");
                continue;
            }
        };

        if let Some(configmap) = event {
            if let Err(err) = handle_event(&ctx, configmap).await {
                error!({ err = &err as &dyn StdError }, "failed to handle event");
                continue;
            };
        } else {
            sleep(Duration::from_secs(1)).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
        .unwrap();

    let client = Client::try_default().await?;
    let api: Api<ConfigMap> = Api::default_namespaced(client);
    let manager = ConfigManager::new(&opts.mount_path);

    let ctx = Context { opts, manager, api };

    match ctx.opts.command.as_ref() {
        Some(&Command::Init) => init(ctx).await,
        _ => run(ctx).await,
    }
}
