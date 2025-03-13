#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

//! The Docker runner is a service that runs on a machine and listens for
//! incoming WebSocket connections from the Patr API. The runner is responsible
//! for creating, updating, and deleting deployments in the given runner.

use std::time::Duration;

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
struct DockerRunner {
	/// The [`Docker`] client.
	docker: Docker,
}

impl RunnerExecutor for DockerRunner {
	type InitializedState = Docker;
	type Settings = DockerSettings;

	async fn initialize(
		_: &RunnerSettings<Self::Settings>,
	) -> Result<Self::InitializedState, RunnerError> {
		let docker = Docker::connect_with_local_defaults()
			.map_err(|_| RunnerError::Unsupported)?
			.negotiate_version()
			.await
			.map_err(|_| RunnerError::Unsupported)?;

		Ok(docker)
	}

	async fn new(_: &RunnerSettings<Self::Settings>, docker: Self::InitializedState) -> Self {
		Self { docker }
	}

	async fn upsert_deployment(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> Result<(), Duration> {
		deployment::upsert(self, deployment, running_details).await
	}

	async fn list_running_deployments<'a>(&self) -> impl Stream<Item = Uuid> + 'a {
		deployment::list_running(self).await
	}

	async fn delete_deployment(&self, id: Uuid) -> Result<(), Duration> {
		deployment::delete(self, id).await
	}

	async fn get_deployment_status(
		&self,
		_deployment_id: Uuid,
	) -> Result<DeploymentStatus, RunnerError> {
		todo!()
	}
}

#[tokio::main]
async fn main() {
	Runner::<DockerRunner>::init()
		.await
		.unwrap()
		.run()
		.await
		.unwrap();
}
