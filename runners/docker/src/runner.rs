use std::{
	collections::{BTreeMap, BTreeSet, HashMap},
	fmt::{self, Display},
	str::FromStr,
	sync::{Arc, Mutex},
	time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use bollard::{
	Docker,
	errors::Error as DockerError,
	models::{
		ConfigSpec,
		EventMessageTypeEnum,
		MountType,
		NetworkCreateRequest,
		Platform,
		SwarmInitRequest,
		SwarmSpec,
		VolumeCreateRequest,
	},
	query_parameters::{
		EventsOptions,
		ListConfigsOptions,
		ListContainersOptions,
		ListNetworksOptions,
		ListServicesOptions,
		ListVolumesOptions,
		RemoveVolumeOptions,
	},
	service::ServiceSpecUpdateConfigOrderEnum,
};
use futures::{Stream, StreamExt, stream};
use models::api::workspace::{deployment::*, runner::*};
use serde::Serialize;
use time::OffsetDateTime;

use crate::prelude::*;

/// An object the Docker runner manages, by kind and name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "name")]
pub enum DockerObjectId {
	/// A Swarm config.
	Config(String),
	/// An overlay network.
	Network(String),
	/// A Swarm service.
	Service(String),
	/// A named volume.
	Volume(String),
}

impl Display for DockerObjectId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Config(name) => write!(f, "config/{name}"),
			Self::Network(name) => write!(f, "network/{name}"),
			Self::Service(name) => write!(f, "service/{name}"),
			Self::Volume(name) => write!(f, "volume/{name}"),
		}
	}
}

impl FromStr for DockerObjectId {
	type Err = RunnerError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.split_once('/') {
			Some(("config", name)) => Ok(Self::Config(name.to_string())),
			Some(("network", name)) => Ok(Self::Network(name.to_string())),
			Some(("service", name)) => Ok(Self::Service(name.to_string())),
			Some(("volume", name)) => Ok(Self::Volume(name.to_string())),
			_ => Err(RunnerError::host(format!("invalid Docker object `{s}`"))),
		}
	}
}

/// What a Docker object should look like. Never `Debug`-printed: configs hold
/// file contents and the Alloy config holds the runner's API token.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DockerSpec {
	/// A config with this content.
	Config {
		/// The file's content.
		data: Vec<u8>,
	},
	/// An overlay network.
	Network {
		/// Whether the network has IPv6. An existing network can't change it,
		/// so a mismatch is only warned about.
		ipv6: bool,
	},
	/// A named volume.
	Volume,
	/// A service.
	Service(Box<ServiceTemplate>),
}

/// The kinds of services the runner runs.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServiceTemplate {
	/// A deployment's service.
	Deployment(DeploymentService),
	/// The Caddy ingress.
	Ingress(IngressService),
	/// The Cloudflare tunnel, for private runners.
	Tunnel {
		/// The hash of the tunnel token, so a new token changes the spec. The
		/// token itself is read from its config when the service is applied.
		token_hash: String,
	},
	/// The Grafana Alloy log and metric collector.
	Alloy {
		/// The config holding Alloy's configuration.
		config: DockerObjectId,
	},
}

/// What a deployment's service is built from. Leaves out what doesn't affect
/// the running service (its name, `deploy_on_push`, `max_horizontal_scale`,
/// `machine_type`), so changing those doesn't restart it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentService {
	/// The deployment the service runs.
	pub deployment_id: Uuid,
	/// Where the image comes from.
	pub registry: DeploymentRegistry,
	/// The image tag.
	pub image_tag: String,
	/// The digest the deployment is pinned to, if any.
	pub current_live_digest: Option<String>,
	/// The environment variables.
	pub env: BTreeMap<String, EnvValue>,
	/// How many tasks run.
	pub replicas: u16,
	/// The probe that checks the deployment started.
	pub startup_probe: Option<DeploymentProbe>,
	/// The probe that checks the deployment is alive.
	pub liveness_probe: Option<DeploymentProbe>,
	/// The configs mounted into the container, by path.
	pub config_mounts: BTreeMap<String, DockerObjectId>,
	/// The volumes mounted into the container, by path.
	pub volume_mounts: BTreeMap<String, DockerObjectId>,
	/// The order old and new tasks are swapped in an update.
	pub update_order: ServiceSpecUpdateConfigOrderEnum,
}

/// An environment variable's value.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum EnvValue {
	/// A plain value.
	Plain(String),
	/// A secret, resolved when the service is applied. Its version is part of
	/// the spec, so rotating it re-applies the service.
	Secret {
		/// The secret.
		secret_id: Uuid,
		/// When the secret's value last changed, if known.
		last_updated: Option<OffsetDateTime>,
	},
}

/// What the Caddy ingress is built from.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressService {
	/// The config holding the main Caddyfile.
	pub base_config: DockerObjectId,
	/// The configs holding the error pages, by the path they're mounted at.
	pub error_pages: BTreeMap<String, DockerObjectId>,
	/// The deployments' and managed URLs' route configs. They belong to other
	/// owners, so the ones that don't exist yet are left out when applying,
	/// and the service is stamped with a hash that makes the next reconcile
	/// try again.
	pub imports: BTreeSet<DockerObjectId>,
	/// The volume Caddy keeps its certificates in.
	pub data_volume: DockerObjectId,
	/// The host ports for HTTP and HTTPS, for public runners.
	pub published_ports: Option<(u16, u16)>,
}

/// A Patr runner that uses Docker to run deployments.
#[derive(Debug, Clone)]
pub struct DockerRunner {
	/// The [`Docker`] client.
	pub docker: Docker,
	/// The runner settings.
	pub settings: RunnerSettings<DockerSettings>,
	/// The platforms each digest-pinned external image supports, so pinned
	/// images aren't looked up again on every apply.
	pub platforms: Arc<Mutex<HashMap<String, Vec<Platform>>>>,
}

impl DockerRunner {
	/// The owner of a service, from its name, or `None` if it isn't Patr's.
	/// Docker's events carry the service's name, not its labels.
	pub fn service_owner(name: &str) -> Option<Owner> {
		if let Some(deployment_id) = name.strip_prefix("patr-") &&
			let Ok(deployment_id) = deployment_id.parse::<Uuid>()
		{
			return Some(Owner::Deployment(deployment_id));
		}
		[
			constants::INGRESS_SERVICE_NAME,
			constants::TUNNEL_SERVICE_NAME,
			constants::ALLOY_SERVICE_NAME,
		]
		.contains(&name)
		.then_some(Owner::System)
	}
}

impl RunnerExecutor for DockerRunner {
	type Id = DockerObjectId;
	type Inputs = Option<String>;
	type Settings = DockerSettings;
	type Spec = DockerSpec;

	fn runner_exposure_type(settings: &RunnerSettings<Self::Settings>) -> RunnerExposureType {
		settings.data.runner_exposure_type.clone()
	}

	async fn initialize(settings: &RunnerSettings<Self::Settings>) -> Result<Self, RunnerError> {
		// Honors the DOCKER_HOST env var, falling back to the platform-default
		// local socket when unset.
		let docker = Docker::connect_with_defaults()
			.map_err(RunnerError::host)?
			.negotiate_version()
			.await
			.map_err(RunnerError::host)?;

		let swarm = docker.inspect_swarm().await.ok();

		if swarm.and_then(|swarm| swarm.id).is_none() {
			docker
				.init_swarm({
					SwarmInitRequest {
						listen_addr: Some(settings.data.docker_swarm_listen_addr.clone()),
						spec: Some(SwarmSpec {
							labels: Some([("managed-by".to_string(), "patr".to_string())].into()),
							..Default::default()
						}),
						..Default::default()
					}
				})
				.await
				.map_err(RunnerError::host)?;
		}

		Ok(Self {
			docker,
			settings: settings.clone(),
			platforms: Arc::new(Mutex::new(HashMap::new())),
		})
	}

	async fn update_ingress_token(&self, token: String) -> Result<bool, RunnerError> {
		ingress::update_tunnel_token(&self.docker, token).await
	}

	async fn inputs(&self, owner: Owner) -> Result<Self::Inputs, RunnerError> {
		if owner != Owner::System || !self.settings.data.runner_exposure_type.is_private() {
			return Ok(None);
		}
		Ok(ingress::tunnel_token(&self.docker)
			.await?
			.map(utils::sha256_hex))
	}

	fn expected(
		&self,
		owner: Owner,
		local: &LocalState,
		inputs: &Self::Inputs,
	) -> Result<ExpectedGraph<Self::Id, Self::Spec>, RunnerError> {
		match owner {
			Owner::Deployment(deployment_id) => {
				Ok(deployment::expected(self, deployment_id, local))
			}
			Owner::ManagedUrl(managed_url_id) => managed_url::expected(self, managed_url_id, local),
			Owner::System => Ok(ingress::expected(self, local, inputs.as_deref())),
		}
	}

	async fn list(&self, owner: Option<Owner>) -> Result<Vec<Object<Self::Id>>, RunnerError> {
		let filters = HashMap::from([(
			String::from("label"),
			vec![
				String::from("managed-by=patr"),
				match owner {
					Some(owner) => format!("patr.owner={owner}"),
					None => String::from("patr.owner"),
				},
			],
		)]);
		let object = |id, labels: Option<&HashMap<String, String>>, depends_on| {
			let labels = labels?;
			Some(Object {
				id,
				owner: labels.get("patr.owner")?.parse::<Owner>().ok()?,
				hash: labels.get("patr.specHash")?.clone(),
				depends_on,
			})
		};

		let mut objects = Vec::new();

		for config in self
			.docker
			.list_configs(Some(ListConfigsOptions {
				filters: Some(filters.clone()),
			}))
			.await
			.map_err(RunnerError::host)?
		{
			let Some(spec) = config.spec else {
				continue;
			};
			let Some(name) = spec.name else {
				continue;
			};
			objects.extend(object(
				DockerObjectId::Config(name),
				spec.labels.as_ref(),
				BTreeSet::new(),
			));
		}

		for network in self
			.docker
			.list_networks(Some(ListNetworksOptions {
				filters: Some(filters.clone()),
			}))
			.await
			.map_err(RunnerError::host)?
		{
			let Some(name) = network.name else {
				continue;
			};
			objects.extend(object(
				DockerObjectId::Network(name),
				network.labels.as_ref(),
				BTreeSet::new(),
			));
		}

		for volume in self
			.docker
			.list_volumes(Some(ListVolumesOptions {
				filters: Some(filters.clone()),
			}))
			.await
			.map_err(RunnerError::host)?
			.volumes
			.unwrap_or_default()
		{
			objects.extend(object(
				DockerObjectId::Volume(volume.name),
				Some(&volume.labels),
				BTreeSet::new(),
			));
		}

		for service in self
			.docker
			.list_services(Some(ListServicesOptions {
				filters: Some(filters),
				status: None,
			}))
			.await
			.map_err(RunnerError::host)?
		{
			let Some(spec) = service.spec else {
				continue;
			};
			let Some(name) = spec.name else {
				continue;
			};
			let container_spec = spec
				.task_template
				.and_then(|task| task.container_spec)
				.unwrap_or_default();
			let depends_on = container_spec
				.configs
				.unwrap_or_default()
				.into_iter()
				.filter_map(|config| config.config_name.map(DockerObjectId::Config))
				.chain(
					container_spec
						.mounts
						.unwrap_or_default()
						.into_iter()
						.filter(|mount| mount.typ == Some(MountType::VOLUME))
						.filter_map(|mount| mount.source.map(DockerObjectId::Volume)),
				)
				.collect();
			objects.extend(object(
				DockerObjectId::Service(name),
				spec.labels.as_ref(),
				depends_on,
			));
		}

		Ok(objects)
	}

	async fn apply(
		&self,
		owner: Owner,
		id: &Self::Id,
		spec: &Self::Spec,
		hash: &str,
	) -> Result<(), RunnerError> {
		match (id, spec) {
			(DockerObjectId::Config(name), DockerSpec::Config { data }) => {
				match self
					.docker
					.create_config(ConfigSpec {
						name: Some(name.clone()),
						labels: Some(utils::object_labels(owner, hash)),
						// Docker's POST /configs/create expects `Data`
						// base64-encoded, and bollard doesn't encode it.
						data: Some(BASE64.encode(data)),
						templating: None,
					})
					.await
				{
					// The name holds the content's hash, so an existing config
					// already has this content.
					Ok(_) |
					Err(DockerError::DockerResponseServerError {
						status_code: 409, ..
					}) => Ok(()),
					Err(err) => Err(RunnerError::host(err)),
				}
			}
			(DockerObjectId::Network(name), DockerSpec::Network { ipv6 }) => {
				match self.docker.inspect_network(name, None).await {
					Ok(network) => {
						if network.enable_ipv6 != Some(*ipv6) {
							warn!(
								network = %name,
								ipv6,
								"The network's IPv6 setting differs from the runner's, and an overlay network can't be changed in place"
							);
						}
						Ok(())
					}
					Err(DockerError::DockerResponseServerError {
						status_code: 404, ..
					}) => {
						self.docker
							.create_network(NetworkCreateRequest {
								name: name.clone(),
								driver: Some(String::from("overlay")),
								ingress: Some(false),
								config_only: Some(false),
								enable_ipv4: Some(true),
								enable_ipv6: Some(*ipv6),
								labels: Some(utils::object_labels(owner, hash)),
								..Default::default()
							})
							.await
							.map_err(RunnerError::host)?;
						Ok(())
					}
					Err(err) => Err(RunnerError::host(err)),
				}
			}
			(DockerObjectId::Volume(name), DockerSpec::Volume) => {
				match self.docker.inspect_volume(name).await {
					Ok(_) => Ok(()),
					Err(DockerError::DockerResponseServerError {
						status_code: 404, ..
					}) => {
						self.docker
							.create_volume(VolumeCreateRequest {
								name: Some(name.clone()),
								labels: Some(utils::object_labels(owner, hash)),
								..Default::default()
							})
							.await
							.map_err(RunnerError::host)?;
						Ok(())
					}
					Err(err) => Err(RunnerError::host(err)),
				}
			}
			(DockerObjectId::Service(name), DockerSpec::Service(template)) => {
				match template.as_ref() {
					ServiceTemplate::Deployment(service) => {
						deployment::apply(self, owner, name, service, hash).await
					}
					ServiceTemplate::Ingress(service) => {
						ingress::apply_ingress(self, owner, name, service, hash).await
					}
					ServiceTemplate::Tunnel { .. } => {
						ingress::apply_tunnel(self, owner, name, hash).await
					}
					ServiceTemplate::Alloy { config } => {
						alloy::apply(self, owner, name, config, hash).await
					}
				}
			}
			_ => Err(RunnerError::host(format!(
				"`{id}` doesn't match the kind of its spec"
			))),
		}
	}

	fn finalizers(&self, id: &Self::Id) -> Vec<FinalizerKind> {
		match id {
			DockerObjectId::Volume(_) => vec![FinalizerKind::Volume],
			_ => Vec::new(),
		}
	}

	async fn in_use(&self, id: &Self::Id) -> Result<bool, RunnerError> {
		match id {
			DockerObjectId::Config(name) => Ok(self
				.docker
				.list_services(None::<ListServicesOptions>)
				.await
				.map_err(RunnerError::host)?
				.into_iter()
				.filter_map(|service| service.spec?.task_template?.container_spec?.configs)
				.flatten()
				.any(|config| config.config_name.as_ref() == Some(name))),
			DockerObjectId::Volume(name) => Ok(!self
				.docker
				.list_containers(Some(ListContainersOptions {
					all: true,
					filters: Some(HashMap::from([(
						String::from("volume"),
						vec![name.clone()],
					)])),
					..Default::default()
				}))
				.await
				.map_err(RunnerError::host)?
				.is_empty()),
			DockerObjectId::Network(_) | DockerObjectId::Service(_) => Ok(false),
		}
	}

	async fn delete(&self, id: &Self::Id) -> Result<DeleteOutcome, RunnerError> {
		let result = match id {
			DockerObjectId::Config(name) => self.docker.delete_config(name).await,
			DockerObjectId::Network(name) => self.docker.remove_network(name).await,
			DockerObjectId::Service(name) => self.docker.delete_service(name).await,
			DockerObjectId::Volume(name) => {
				self.docker
					.remove_volume(name, None::<RemoveVolumeOptions>)
					.await
			}
		};
		match result {
			Ok(()) |
			Err(DockerError::DockerResponseServerError {
				status_code: 404, ..
			}) => Ok(DeleteOutcome::Deleted),
			Err(DockerError::DockerResponseServerError {
				status_code: 400 | 403 | 409,
				..
			}) => Ok(DeleteOutcome::InUse),
			Err(err) => Err(RunnerError::host(err)),
		}
	}

	async fn observe(&self, deployment_id: Uuid) -> Result<ObservedDeploymentStatus, RunnerError> {
		deployment::observe(self, deployment_id).await
	}

	fn watch(&self) -> impl Stream<Item = Owner> + Send + use<> {
		// The events that change what `observe` reports, or a service. Health
		// checks run as execs, so with `exec_*` events every health check would
		// reconcile its deployment.
		let options = |since: i64| EventsOptions {
			since: Some(since.to_string()),
			until: None,
			filters: Some(HashMap::from([
				(
					String::from("type"),
					vec![String::from("container"), String::from("service")],
				),
				(
					String::from("event"),
					[
						"start",
						"die",
						"oom",
						"health_status",
						"create",
						"update",
						"remove",
					]
					.map(String::from)
					.to_vec(),
				),
			])),
		};
		let since = OffsetDateTime::now_utc().unix_timestamp();

		stream::unfold(
			(
				self.docker.clone(),
				Box::pin(self.docker.events(Some(options(since)))),
				since,
				Duration::from_secs(1),
			),
			move |(docker, mut events, mut since, mut backoff)| async move {
				loop {
					let event = match events.next().await {
						Some(Ok(event)) => event,
						// `None` if Docker ended the stream
						error => {
							warn!(
								?error,
								"Docker's event stream stopped, reconnecting in {backoff:?}"
							);
							tokio::time::sleep(backoff).await;
							backoff = (backoff * 2).min(Duration::from_secs(60));
							// Docker replays the events since the last one seen
							events = Box::pin(docker.events(Some(options(since))));
							continue;
						}
					};
					backoff = Duration::from_secs(1);
					since = event.time.unwrap_or(since);

					let Some(attributes) = event.actor.and_then(|actor| actor.attributes) else {
						continue;
					};
					let service = match event.typ {
						Some(EventMessageTypeEnum::CONTAINER) => {
							attributes.get("com.docker.swarm.service.name")
						}
						Some(EventMessageTypeEnum::SERVICE) => attributes.get("name"),
						_ => None,
					};
					if let Some(owner) = service.and_then(|name| Self::service_owner(name)) {
						return Some((owner, (docker, events, since, backoff)));
					}
				}
			},
		)
	}
}
