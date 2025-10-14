use super::*;

const DEFAULT_HEADSCALE_IMAGE: &str = "headscale/headscale:v0.26.1";

#[derive(Deserialize)]
#[serde(default)]
struct Config {
    listen_addr: SocketAddr,
    metrics_listen_addr: SocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8080".parse().unwrap(),
            metrics_listen_addr: "0.0.0.0:9090".parse().unwrap(),
        }
    }
}

struct Volumes {
    config: Volume,
    keys: Volume,
    tls: Volume,
}

struct Ports {
    http: u16,
    metrics: u16,
    derp: u16,
}

impl Default for Ports {
    fn default() -> Self {
        Self {
            http: 8080,
            metrics: 9090,
            derp: 3478,
        }
    }
}

fn gen_private_key() -> String {
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::rng().fill_bytes(&mut buf);
    format!("privkey:{}", hex::encode(buf))
}

impl Headscale {
    fn common_labels(&self, name: impl ToString) -> impl Iterator<Item = (&'static str, String)> {
        let name = name.to_string();
        let manager = env!("CARGO_PKG_NAME").to_string();
        let version = env!("CARGO_PKG_VERSION").to_string();
        let instance = format!("headscale-{name}");
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

    fn get_ports(&self) -> Ports {
        let value = self.spec.config.clone();
        let config = serde_json::from_value::<Config>(value).unwrap();

        Ports {
            http: config.listen_addr.port(),
            metrics: config.metrics_listen_addr.port(),
            derp: 3478,
        }
    }

    fn render_volumes(&self, config: &ConfigMap, keys: &Secret) -> Volumes {
        let config_name = config.name_unchecked();
        let keys_name = &keys.name_unchecked();
        let tls_name = &self
            .spec
            .tls
            .existing_secret
            .clone()
            .expect("valid secret name");

        let config = Volume::configmap("config", ConfigMapVolumeSource::new(config_name));
        let keys = Volume::secret("keys", SecretVolumeSource::secret_name(keys_name));
        let tls = Volume::secret("tls", SecretVolumeSource::secret_name(tls_name));

        Volumes { config, keys, tls }
    }

    pub fn stateful_set_name(&self) -> String {
        format!("headscale-{}", self.name_unchecked())
    }

    fn render_stateful_set(&self, ports: &Ports, volumes: Volumes) -> StatefulSet {
        let name = self.stateful_set_name();
        let namespace = self.namespace().unwrap_or_default();
        let owner_ref = self.owner_ref(&()).unwrap_or_default();

        let pod_spec = PodSpec::container(
            Container::new("headscale")
                .image(DEFAULT_HEADSCALE_IMAGE)
                .command(["headscale", "serve"])
                .ports([
                    ContainerPort::tcp(ports.http).name("http"),
                    ContainerPort::tcp(ports.metrics).name("metrics"),
                    ContainerPort::udp(ports.derp).name("derp"),
                ])
                .env(self.spec.deployment.env.clone())
                .volume_mounts([
                    VolumeMount::new("/etc/headscale/config.yaml", &volumes.config)
                        .sub_path("config.yaml")
                        .read_only(),
                    VolumeMount::new("/etc/headscale/tls", &volumes.tls).read_only(),
                    VolumeMount::new("/var/lib/headscale", &volumes.keys).read_only(),
                ]),
        )
        .volumes([volumes.config, volumes.tls, volumes.keys]);

        StatefulSet::new(&name)
            .namespace(&namespace)
            .owner(owner_ref)
            .labels(self.common_labels(&name))
            .replicas(1)
            .match_labels([("app.kubernetes.io/name", &name)])
            .template(
                PodTemplateSpec::new()
                    .labels(self.common_labels(&name))
                    .pod_spec(pod_spec),
            )
    }

    fn render_secret(&self) -> Secret {
        let owner_ref = self.owner_ref(&()).unwrap_or_default();
        let name = format!("headscale-{}-keys", self.name_unchecked());
        let namespace = self.namespace().unwrap_or_default();

        Secret::new(&name)
            .namespace(&namespace)
            .labels(self.common_labels(&name))
            .owner(owner_ref)
            .string_data([
                ("derp_server_private.key", gen_private_key()),
                ("noise_private.key", gen_private_key()),
            ])
    }

    fn render_configmap(&self) -> ConfigMap {
        let name = format!("headscale-{}-config", self.name_unchecked());
        let namespace = self.namespace().unwrap_or_default();
        let owner_ref = self.owner_ref(&()).unwrap_or_default();

        ConfigMap::new(&name)
            .namespace(&namespace)
            .labels(self.common_labels(&name))
            .owner(owner_ref)
            .data([(
                "config.yaml",
                serde_yaml::to_string(&self.spec.config).unwrap(),
            )])
    }

    fn render_service(&self, ports: &Ports, selector_name: impl ToString) -> Service {
        let name = format!("headscale-{}-service", self.name_unchecked());
        let namespace = self.namespace().unwrap();
        let owner_ref = self.owner_ref(&()).unwrap_or_default();

        Service::cluster_ip(
            &name,
            [
                ServicePort::tcp("https", ports.http),
                ServicePort::tcp("metrics", ports.metrics),
                ServicePort::udp("derp", ports.derp),
            ],
        )
        .namespace(&namespace)
        .labels(self.common_labels(&name))
        .owner(owner_ref)
        .selector([("app.kubernetes.io/name", selector_name)])
    }
}

#[kubus(event = Apply, finalizer = "headscale.juliamertz.dev/headscale-finalizer")]
pub async fn deploy_headscale(
    headscale: Arc<Headscale>,
    ctx: Arc<Context<State>>,
) -> Result<(), Error> {
    let client = ctx.client.clone();

    let ports = headscale.get_ports();
    let keys = headscale.render_secret();
    let config = headscale.render_configmap();
    let volumes = headscale.render_volumes(&config, &keys);
    let stateful_set = headscale.render_stateful_set(&ports, volumes);
    let service = headscale.render_service(&ports, stateful_set.name_unchecked());

    keys.apply_if_not_exists(&client).await?;
    config.apply(&client).await?;
    stateful_set.apply(&client).await?;
    service.apply(&client).await?;

    Ok(())
}

#[kubus(event = Delete, finalizer = "headscale.juliamertz.dev/headscale-finalizer")]
pub async fn cleanup_headscale(
    headscale: Arc<Headscale>,
    ctx: Arc<Context<State>>,
) -> Result<(), Error> {
    let client = &ctx.client;
    let name = headscale.metadata.name.clone().unwrap();
    let namespace = headscale.metadata.namespace.clone().unwrap();

    tracing::info!("deleting headscale {name} from {namespace}");

    let ports = headscale.get_ports();
    let keys = headscale.render_secret();
    let config = headscale.render_configmap();
    let volumes = headscale.render_volumes(&config, &keys);
    let stateful_set = headscale.render_stateful_set(&ports, volumes);
    let service = headscale.render_service(&ports, stateful_set.name_unchecked());

    stateful_set.delete(client).await?;
    service.delete(client).await?;
    config.delete(client).await?;
    keys.delete(client).await?;

    Ok(())
}
