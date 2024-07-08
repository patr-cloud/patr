#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

//! The Docker runner is a service that runs on a machine and listens for
//! incoming WebSocket connections from the Patr API. The runner is responsible
//! for creating, updating, and deleting deployments in the given runner.

use common::Runner;

/// The configuration for the runner.
mod config;
/// The module to handle the creation, updating, and deletion of resources.
mod docker;

struct DockerRunner;

impl RunnerExecutor for DockerRunner {
	type Resource = docker::DockerResource;

	fn new() -> Self {
		Self
	}

	async fn create(&self, resource: Self::Resource) -> Result<(), Error> {
		resource.create().await
	}

	async fn update(&self, resource: Self::Resource) -> Result<(), Error> {
		resource.update().await
	}

	async fn delete(&self, resource: Self::Resource) -> Result<(), Error> {
		resource.delete().await
	}
}

#[tokio::main]
async fn main() {
	let settings = config::get_runner_settings();

	Runner::new(DockerRunner).await.run().await;
}
