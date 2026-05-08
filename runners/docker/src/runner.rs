use std::{collections::HashMap, sync::Arc, time::Duration};

use bollard::{
	Docker,
	models::{NetworkCreateRequest, SwarmInitRequest, SwarmSpec},
};
use futures::Stream;
use models::api::workspace::{deployment::*, runner::*};
use tokio::sync::Mutex;

use crate::prelude::*;

/// Shared state passed from `initialize` into every cloned [`DockerRunner`].
///
/// `ingress_lock` serializes writes to the single `patr-ingress` swarm service.
/// Without it, concurrent deployment actors race on swarmkit's optimistic
/// version index and one of them gets "update out of sequence".
#[derive(Debug, Clone)]
pub struct DockerRunnerState {
	/// The [`Docker`] client (cheaply clonable; internally an `Arc`).
	pub docker: Docker,
	/// Mutex guarding all reads-then-writes of the shared `patr-ingress`
	/// service.
	pub ingress_lock: Arc<Mutex<()>>,
}

/// A Patr runner that uses Docker to run deployments.
#[derive(Debug, Clone)]
pub struct DockerRunner {
	/// The [`Docker`] client.
	pub docker: Docker,
	/// The runner settings.
	pub settings: RunnerSettings<DockerSettings>,
	/// Shared lock to serialize ingress-service writes across all actor clones.
	pub ingress_lock: Arc<Mutex<()>>,
}

impl RunnerExecutor for DockerRunner {
	type InitializedState = DockerRunnerState;
	type Settings = DockerSettings;

	fn runner_exposure_type(settings: &RunnerSettings<Self::Settings>) -> RunnerExposureType {
		settings.data.runner_exposure_type.clone()
	}

	async fn initialize(
		settings: &RunnerSettings<Self::Settings>,
	) -> Result<Self::InitializedState, RunnerError> {
		let docker = Docker::connect_with_local_defaults()
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

		let network = docker
			.inspect_network(constants::INGRESS_NETWORK_NAME, None)
			.await
			.ok();

		if network.and_then(|network| network.id).is_none() {
			docker
				.create_network(NetworkCreateRequest {
					name: String::from(constants::INGRESS_NETWORK_NAME),
					driver: Some(String::from("overlay")),
					ingress: Some(false),
					config_only: Some(false),
					enable_ipv4: Some(true),
					enable_ipv6: Some(true),
					labels: Some(HashMap::from([
						("managed-by".to_string(), "patr".to_string()),
						(
							"patr.version".to_string(),
							constants::PATR_VERSION.to_string(),
						),
					])),
					..Default::default()
				})
				.await
				.map_err(RunnerError::host)?;
		}

		// None of this is required if the runner is not using a tunnel
		if settings.data.runner_exposure_type.requires_tunnel() {
			let RunnerMode::Managed {
				workspace_id,
				runner_id,
				api_token,
				user_agent,
			} = settings.mode.clone()
			else {
				// If the runner is running in self-hosted mode, throw an error if the runner is
				// set to use a tunnel, since tunnels are only supported in managed mode
				debug!(concat!(
					"Runner is running in self-hosted mode. ",
					"Please expose the runner to the internet manually, ",
					"as tunnels are not supported in self-hosted mode."
				));
				return Err(RunnerError::Unsupported);
			};

			let existing_tunnel_token = docker
				.inspect_config(constants::TUNNEL_TOKEN_CONFIG_NAME)
				.await
				.ok()
				.and_then(|config| config.spec)
				.and_then(|spec| spec.data);

			let new_tunnel_token = client::make_request(
				ApiRequest::<GetIngressTokenForRunnerRequest>::builder()
					.path(GetIngressTokenForRunnerPath {
						workspace_id,
						runner_id,
					})
					.headers(GetIngressTokenForRunnerRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.build(),
			)
			.await
			.map_err(|err| err.body.error)
			.map(|response| response.body.token);

			match (existing_tunnel_token, new_tunnel_token) {
				(None, Err(err)) => {
					error!(
						"Cannot get tunnel token for runner: {err}. {}",
						"Are you connected to the internet?"
					);
					return Err(RunnerError::UpstreamServerError(err));
				}
				(Some(_), Err(err)) => {
					warn!("Cannot get tunnel token for runner: {err}");
					warn!(concat!(
						"Using existing tunnel token, but this may cause ",
						"connectivity issues if the token has changed or is invalid."
					));
				}
				(Some(existing), Ok(new)) if existing == new => {
					debug!("Tunnel token has not changed, using existing token");
				}
				(Some(_), Ok(new)) | (None, Ok(new)) => {
					ingress::update_ingress_tunnel_token(&docker, new).await?;
				}
			}
		}

		// Setup ingress, if it doesn't exist
		ingress::update_ingress_configs(&docker, settings).await?;

		// Setup Alloy log collector for managed mode
		if settings.mode.is_managed() {
			alloy::update_alloy_service(&docker, settings).await?;
		}

		Ok(DockerRunnerState {
			docker,
			ingress_lock: Arc::new(Mutex::new(())),
		})
	}

	async fn new(settings: &RunnerSettings<Self::Settings>, state: Self::InitializedState) -> Self {
		Self {
			docker: state.docker,
			settings: settings.clone(),
			ingress_lock: state.ingress_lock,
		}
	}

	async fn upsert_deployment(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> Result<(), RunnerError> {
		deployment::upsert(self, deployment, running_details).await?;

		let _guard = self.ingress_lock.lock().await;
		ingress::update_ingress_configs(&self.docker, &self.settings).await
	}

	async fn list_running_deployments<'a>(&self) -> impl Stream<Item = Uuid> + 'a {
		deployment::list_running(self).await
	}

	async fn delete_deployment(&self, id: Uuid) -> Result<(), RunnerError> {
		deployment::delete(self, id).await?;

		let _guard = self.ingress_lock.lock().await;
		ingress::delete_deployment_config(&self.docker, &self.settings, id).await
	}

	async fn get_deployment_status(
		&self,
		deployment_id: Uuid,
	) -> Result<DeploymentStatus, RunnerError> {
		deployment::get_status(self, deployment_id).await
	}

	async fn next_deployment_status(
		&self,
		deployment_id: Uuid,
	) -> Result<DeploymentStatus, RunnerError> {
		tokio::time::sleep(Duration::from_secs(5)).await;
		self.get_deployment_status(deployment_id).await
	}

	async fn upsert_managed_url(
		&self,
		managed_url_id: Uuid,
		host: String,
		path: String,
		deployment_id: Uuid,
		port: u16,
	) -> Result<(), RunnerError> {
		managed_url::upsert(self, managed_url_id, host, path, deployment_id, port).await
	}

	async fn delete_managed_url(&self, managed_url_id: Uuid) -> Result<(), RunnerError> {
		managed_url::delete(self, managed_url_id).await
	}

	async fn list_running_managed_urls(&self) -> Result<Vec<Uuid>, RunnerError> {
		managed_url::list_running(self).await
	}
}
