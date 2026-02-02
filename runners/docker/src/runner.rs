use std::{collections::HashMap, time::Duration};

use bollard::{
	Docker,
	models::{NetworkCreateRequest, SwarmInitRequest, SwarmSpec},
};
use futures::Stream;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// A Patr runner that uses Docker to run deployments.
#[derive(Debug, Clone)]
pub struct DockerRunner {
	/// The [`Docker`] client.
	pub docker: Docker,
	/// The runner settings.
	pub settings: DockerSettings,
}

impl RunnerExecutor for DockerRunner {
	type InitializedState = Docker;
	type Settings = DockerSettings;

	fn runner_exposure_type() -> RunnerExposureType {
		RunnerExposureType::Private
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
					let mut request = SwarmInitRequest::default();

					request.listen_addr = Some(settings.data.docker_swarm_listen_addr.clone());
					request.spec = Some(SwarmSpec {
						labels: Some([("managed-by".to_string(), "patr".to_string())].into()),
						..Default::default()
					});

					request
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
					scope: None,
					internal: Some(true),
					attachable: None,
					ingress: Some(false),
					config_only: Some(false),
					config_from: None,
					ipam: None,
					enable_ipv4: Some(true),
					enable_ipv6: Some(true),
					options: None,
					labels: Some(HashMap::from([(
						"managed-by".to_string(),
						"patr".to_string(),
					)])),
				})
				.await
				.map_err(RunnerError::host)?;
		}

		// Setup ingress, if it doesn't exist
		ingress::update_ingress_configs(&docker, &settings.data).await?;

		Ok(docker)
	}

	async fn new(
		settings: &RunnerSettings<Self::Settings>,
		docker: Self::InitializedState,
	) -> Self {
		Self {
			docker,
			settings: settings.data.clone(),
		}
	}

	async fn upsert_deployment(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> Result<(), RunnerError> {
		deployment::upsert(self, deployment, running_details).await?;

		ingress::update_ingress_configs(&self.docker, &self.settings).await
	}

	async fn list_running_deployments<'a>(&self) -> impl Stream<Item = Uuid> + 'a {
		deployment::list_running(self).await
	}

	async fn delete_deployment(&self, id: Uuid) -> Result<(), RunnerError> {
		deployment::delete(self, id).await?;

		ingress::update_ingress_configs(&self.docker, &self.settings).await
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
}
