#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

//! This project generates a controller that can be installed in the user's
//! Kubernetes cluster. Each controller will be responsible for managing the
//! respective cluster. The controller will periodically check with the API and
//! make sure that the cluster's state is up to date with the API's state.

use std::{marker::PhantomData, str::FromStr, sync::Arc};

use ::models::{
	api::workspace::runner::*,
	prelude::UserAgent,
	utils::{BearerToken, WebSocketUpgrade},
	ApiRequest,
};
use app::AppState;
use futures::StreamExt;
use prelude::*;
use tokio::{sync::broadcast, time::Duration};

/// A prelude that re-exports commonly used items.
pub mod prelude {
	use models::prelude::*;
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

	let (patr_update_sender, patr_update_receiver) = broadcast::channel::<()>(100);

	client::stream_request(
		ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
			.path(StreamRunnerDataForWorkspacePath {
				workspace_id: state.workspace_id,
				runner_id: state.region_id,
			})
			.query(())
			.headers(StreamRunnerDataForWorkspaceRequestHeaders {
				authorization: BearerToken::from_str(state.patr_token.as_str()).unwrap(),
				user_agent: UserAgent::from_static("deployment-controller"),
			})
			.body(WebSocketUpgrade::new())
			.build(),
	)
	.await
	.unwrap()
	.for_each(|deployment| async {
		_ = patr_update_sender.send(());
	})
	.await;

	let (reconcile_all_deployments, deployment_controller_task) =
		deployment::start_controller(state.client.clone(), state.clone(), patr_update_receiver);

	loop {
		tokio::select! {
			_ = async {
				// Ever 1 hour, reconcile everything
				tokio::time::sleep(Duration::from_secs(3600)).await;
				_ = reconcile_all_deployments.send(());
			} => {},
			_ = exit_signal() => {
				tracing::info!("Received SIGINT, shutting down");

				// Wait for all existing controllers to finish
				_ = deployment_controller_task.await;

				// Break out of the loop and exit
				break;
			}
		}
	}
}

async fn exit_signal() {
	tokio::signal::ctrl_c()
		.await
		.expect("Failed to listen for SIGINT")
}
