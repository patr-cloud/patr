#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! This project generates a controller that can be installed in the user's
//! Kubernetes cluster. Each controller will be responsible for managing the
//! respective cluster. The controller will periodically check with the API and
//! make sure that the cluster's state is up to date with the API's state.

use std::sync::Arc;

use app::AppState;
use tokio::time::Duration;

/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{AppError, AppState},
		models::PatrDeployment,
		utils::KubeApiExt,
	};
}

/// All app state that is shared across the entire application. Used to share
/// ApiTokens, backend connections, etc.
mod app;
/// The client used to communicate with the Patr API.
mod client;
/// All the constants used by the controller.
mod constants;
/// All functions and business logic to run a database controller and keep it
/// in sync with the Patr API data.
mod database;
/// All functions and business login to run a deployment controller and keep it
/// in sync with the Patr API data.
mod deployment;
/// All models used by the controller, including CRDs, etc.
mod models;
/// Utility functions used by the controller.
mod utils;

#[tokio::main]
async fn main() {
	let state = Arc::new(AppState::try_default().await);

	let (mut reconcile_all_deployments, deployment_controller_task) =
		deployment::start_controller(state.client.clone(), state.clone());

	loop {
		tokio::select! {
			_ = async {
				// Ever 1 hour, reconcile everything
				tokio::time::sleep(Duration::from_secs(3600)).await;
				_ = reconcile_all_deployments.send(());
			} => {},
			_ = tokio::signal::ctrl_c() => {
				tracing::info!("Received SIGINT, shutting down");

				// Wait for all existing controllers to finish
				_ = deployment_controller_task.await;

				// Break out of the loop and exit
				break;
			}
		}
	}
}
