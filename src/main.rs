use std::net::SocketAddr;
use std::{io::Write, sync::Arc};

use clap::{Parser, Subcommand};
use k8s_openapi::List;
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec};
use k8s_openapi_ext::corev1::{
    ConfigMap, ConfigMapVolumeSource, PodTemplateSpec, Secret, SecretVolumeSource, Service,
    ServicePort, Volume, VolumeMount,
};
use k8s_openapi_ext::*;
use kube::api::DeleteParams;
use kube::{Api, Client, CustomResourceExt, Resource};
use kubus::{ApiExt, Context, Operator, kubus};
use rand::RngCore;
use thiserror::Error;

mod crds;
use crds::*;

const MANAGER: &str = env!("CARGO_PKG_NAME");

const DEFAULT_IMAGE: &str = "headscale/headscale:v0.26.1";

#[derive(Debug, Error)]
enum Error {
    #[error("kube error: {0}")]
    Kube(#[from] kube::Error),
    #[error("kubus error: {0}")]
    Kubus(#[from] kubus::Error),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
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
            list.items = vec![Headscale::crd(), ACLPolicy::crd()];
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
                .run()
                .await?
        }
    };

    Ok(())
}

fn gen_private_key() -> String {
    let mut key_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut key_bytes);
    format!("privkey:{}", hex::encode(key_bytes))
}

fn common_labels(name: impl ToString) -> impl Iterator<Item = (&'static str, String)> {
    let name = name.to_string();
    let manager = MANAGER.to_string();
    let instance = format!("headscale-{name}");
    let version = env!("CARGO_PKG_VERSION").to_string();
    let part_of = "headscale".to_string();
    [
        ("app.kubernetes.io/name", name),
        ("app.kubernetes.io/managed-by", manager),
        ("app.kubernetes.io/instance", instance),
        ("app.kubernetes.io/version", version),
        ("app.kubernetes.io/part-of", part_of),
    ]
    .into_iter()
}

#[derive(Deserialize)]
#[serde(default)]
struct HeadscaleConfig {
    listen_addr: SocketAddr,
    metrics_listen_addr: SocketAddr,
}

impl Default for HeadscaleConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            metrics_listen_addr: "0.0.0.0:9090".parse().unwrap(),
        }
    }
}

#[kubus(event = Apply, finalizer = "kubus.io/headscale-finalizer")]
async fn deploy_headscale(
    headscale: Arc<Headscale>,
    ctx: Arc<Context<State>>,
) -> Result<(), Error> {
    let client = ctx.client.clone();
    let config: HeadscaleConfig = serde_json::from_value(headscale.spec.config.clone()).unwrap();
    let port: i32 = config.listen_addr.port().into();
    let metrics_port: i32 = config.metrics_listen_addr.port().into();
    let derp_port: i32 = 3478;

    tracing::info!("headscale: {headscale:?}");

    let name = headscale.metadata.name.clone().unwrap();
    let namespace = headscale.metadata.namespace.clone().unwrap();
    let object_ref = headscale.object_ref(&());
    let owner_ref = k8s_openapi_ext::owner_reference(object_ref, true, false).unwrap();

    let keys_secret_name = format!("headscale-{name}-keys");
    Secret::new(&keys_secret_name)
        .namespace(&namespace)
        .labels(common_labels(&keys_secret_name))
        .owner(owner_ref.clone())
        .string_data([
            ("derp_server_private.key", gen_private_key()),
            ("noise_private.key", gen_private_key()),
        ])
        .apply_if_not_exists(&client)
        .await?;

    let configmap_name = format!("headscale-{name}-config");
    ConfigMap::new(&configmap_name)
        .namespace(&namespace)
        .labels(common_labels(&configmap_name))
        .owner(owner_ref.clone())
        .data([(
            "config.yaml",
            serde_yaml::to_string(&headscale.spec.config).unwrap(),
        )])
        .apply(&client)
        .await?;

    let config_volume = Volume::configmap("config", ConfigMapVolumeSource::new(&configmap_name));
    let keys_volume = Volume::secret("keys", SecretVolumeSource::secret_name(&keys_secret_name));
    let tls_volume = Volume::secret(
        "tls",
        // TODO: proper integration for headscale
        SecretVolumeSource::secret_name(
            &headscale
                .spec
                .tls
                .existing_secret
                .clone()
                .expect("valid secret name"),
        ),
    );

    let deployment_name = format!("headscale-{name}");
    Deployment::new(&deployment_name)
        .namespace(&namespace)
        .labels(common_labels(&deployment_name))
        .owner(owner_ref.clone())
        .replicas(1)
        .match_labels([("app.kubernetes.io/name", &deployment_name)])
        .template(
            PodTemplateSpec::new()
                .labels(common_labels(&deployment_name))
                .pod_spec(
                    PodSpec::container(
                        Container::new("headscale")
                            .image(DEFAULT_IMAGE)
                            .command(["headscale", "serve"])
                            .ports([
                                ContainerPort::new(port, "TCP").name("http"),
                                ContainerPort::new(metrics_port, "TCP").name("metrics"),
                                ContainerPort::new(derp_port, "UDP").name("derp"),
                            ])
                            .env(headscale.spec.deployment.env.clone())
                            .volume_mounts([
                                VolumeMount::new("/etc/headscale/config.yaml", &config_volume)
                                    .sub_path("config.yaml")
                                    .read_only(),
                                VolumeMount::new("/etc/headscale/tls", &tls_volume).read_only(),
                                VolumeMount::new("/var/lib/headscale", &keys_volume).read_only(),
                            ]),
                    )
                    .volumes([config_volume, keys_volume, tls_volume]),
                ),
        )
        .apply(&client)
        .await?;

    let service_name = format!("headscale-{name}-service");
    Service::cluster_ip(
        &service_name,
        [
            ServicePort::new("https", port).protocol("TCP"),
            ServicePort::new("metrics", metrics_port).protocol("TCP"),
            ServicePort::new("derp", derp_port).protocol("UDP"),
        ],
    )
    .namespace(&namespace)
    .owner(owner_ref.clone())
    .selector([("app.kubernetes.io/name", &deployment_name)])
    .apply(&client)
    .await?;

    Ok(())
}

#[kubus(event = Delete, finalizer = "kubus.io/headscale-finalizer")]
async fn cleanup_headscale(
    headscale: Arc<Headscale>,
    ctx: Arc<Context<State>>,
) -> Result<(), Error> {
    let name = headscale.metadata.name.clone().unwrap();
    let namespace = headscale.metadata.namespace.clone().unwrap();

    tracing::info!("deleting headscale {name} from {namespace}");

    let client = &ctx.client;
    let delete_params = DeleteParams::default();

    let deployment_name = format!("headscale-{name}");
    Api::<Deployment>::namespaced(client.clone(), &namespace)
        .delete(&deployment_name, &delete_params)
        .await?;

    let keys_secret_name = format!("headscale-{name}-keys");
    Api::<Secret>::namespaced(client.clone(), &namespace)
        .delete(&keys_secret_name, &delete_params)
        .await?;

    let configmap_name = format!("headscale-{name}-config");
    Api::<ConfigMap>::namespaced(client.clone(), &namespace)
        .delete(&configmap_name, &delete_params)
        .await?;

    let service_name = format!("headscale-{name}-service");
    Api::<Service>::namespaced(client.clone(), &namespace)
        .delete(&service_name, &delete_params)
        .await?;

    Ok(())
}
