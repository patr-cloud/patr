#![feature(never_type)]

//! The Docker runner is a service that runs on a machine and listens for
//! incoming WebSocket connections from the Patr API. The runner is responsible
//! for creating, updating, and deleting deployments in the given runner.

use docker::prelude::*;

#[tokio::main]
async fn main() -> Result<!, RunnerError> {
	Runner::<DockerRunner>::init().await?.run().await
}
