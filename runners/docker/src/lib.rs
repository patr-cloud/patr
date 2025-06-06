//! The Docker runner is a service that runs on a machine and listens for
//! incoming WebSocket connections from the Patr API. The runner is responsible
//! for creating, updating, and deleting deployments in the given runner.

use bollard::Docker;
use common::prelude::*;
use futures::Stream;
use models::api::workspace::deployment::{Deployment, DeploymentRunningDetails, *};
use serde::{Deserialize, Serialize};

/// All deployment related stuff goes here
mod deployment;

/// The configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerSettings {}

/// A Patr runner that uses Docker to run deployments.
#[derive(Debug, Clone)]
pub struct DockerRunner {
	/// The [`Docker`] client.
	docker: Docker,
}

impl RunnerExecutor for DockerRunner {
	type InitializedState = Docker;
	type Settings = DockerSettings;

	fn runner_exposure_type() -> RunnerExposureType {
		RunnerExposureType::Private
	}

	async fn initialize(
		_: &RunnerSettings<Self::Settings>,
	) -> Result<Self::InitializedState, RunnerError> {
		let docker = Docker::connect_with_local_defaults()
			.map_err(RunnerError::host)?
			.negotiate_version()
			.await
			.map_err(RunnerError::host)?;

		Ok(docker)
	}

	async fn new(_: &RunnerSettings<Self::Settings>, docker: Self::InitializedState) -> Self {
		Self { docker }
	}

	async fn upsert_deployment(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> Result<(), RunnerError> {
		deployment::upsert(self, deployment, running_details).await
	}

	async fn list_running_deployments<'a>(&self) -> impl Stream<Item = Uuid> + 'a {
		deployment::list_running(self).await
	}

	async fn delete_deployment(&self, id: Uuid) -> Result<(), RunnerError> {
		deployment::delete(self, id).await
	}

	async fn get_deployment_status(
		&self,
		deployment_id: Uuid,
	) -> Result<DeploymentStatus, RunnerError> {
		// TODO improve this
		Ok(self
			.docker
			.inspect_container(&deployment_id.to_string(), None)
			.await
			.ok()
			.and_then(|container| container.state)
			.map(|state| {
				if state.running.unwrap_or(false) {
					DeploymentStatus::Running
				} else {
					DeploymentStatus::Stopped
				}
			})
			.unwrap_or(DeploymentStatus::Stopped))
	}
}
