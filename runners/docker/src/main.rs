#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

//! The Docker runner is a service that runs on a machine and listens for
//! incoming WebSocket connections from the Patr API. The runner is responsible
//! for creating, updating, and deleting deployments in the given runner.

/// The configuration for the runner.
mod config;
use std::time::Duration;

/// The module to handle the creation, updating, and deletion of resources.
// mod docker;
use common::prelude::*;
use futures::Stream;
use models::api::workspace::deployment::{Deployment, DeploymentRunningDetails};

struct DockerRunner;

impl RunnerExecutor for DockerRunner {
	type Settings<'s> = ();

	async fn reconcile(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> Result<(), Duration> {
		Ok(())
	}

	fn list_running_deployments(&self) -> impl Stream<Item = Uuid> {
		futures::stream::empty()
	}
}

#[tokio::main]
async fn main() {
	let settings = config::get_runner_settings();

	Runner::new(DockerRunner).await.run().await;
}
