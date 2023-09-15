#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! This project generates a controller that can be installed in the user's
//! Kubernetes cluster. Each controller will be responsible for managing the
//! respective cluster. The controller will periodically check with the API and
//! make sure that the cluster's state is up to date with the API's state.

use std::sync::Arc;

use app::AppState;
use kube::Client;
use tokio::task;

/// All app state that is shared across the entire application. Used to share
/// ApiTokens, backend connections, etc.
mod app;
/// All functions and business login to run a deployment controller and keep it
/// in sync with the Patr API data.
mod deployment;
/// All models used by the controller, including CRDs, requests, responses, etc.
#[allow(clippy::missing_docs_in_private_items)]
mod models;

#[tokio::main]
async fn main() {
	let state = Arc::new(AppState::try_default());

	let client = Client::try_default()
		.await
		.expect("Failed to get kubernetes client details");

	let deployment_task = task::spawn(deployment::start_controller(client.clone(), state.clone()));

	_ = deployment_task.await;
}
