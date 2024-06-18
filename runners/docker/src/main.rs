use std::{collections::HashMap, marker::PhantomData, str::FromStr};

use bollard::container::{Config, CreateContainerOptions, StartContainerOptions};
use config::{RunnerSettings, RunningEnvironment};
use futures::StreamExt;
use models::{api::workspace::runner::*, prelude::*, utils::WebSocketUpgrade};
use tracing::{Dispatch, Level};
use tracing_subscriber::{
	filter::LevelFilter,
	fmt::{format::FmtSpan, Layer as FmtLayer},
	layer::SubscriberExt,
	prelude::*,
};

/// The client for the Patr API to get runner data for a given workspace.
mod client;
/// The configuration for the runner.
mod config;
/// The module to handle the creation, updating, and deletion of resources.
mod docker;
/// The utility functions and types used across the crate.
mod utils;

/// The prelude module contains all the commonly used types and traits that are
/// used across the crate. This is mostly used to avoid having to import a lot
/// of things from different modules.
pub mod prelude {
	pub use models::prelude::*;
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::utils::*;
}

#[tokio::main]
async fn main() {
	let settings = config::get_runner_settings();

	tracing::dispatcher::set_global_default(Dispatch::new(
		tracing_subscriber::registry().with(
			FmtLayer::new()
				.with_span_events(FmtSpan::NONE)
				.event_format(
					tracing_subscriber::fmt::format()
						.with_ansi(true)
						.with_file(false)
						.without_time()
						.compact(),
				)
				.with_filter(
					tracing_subscriber::filter::Targets::new()
						.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
						.with_target("models", LevelFilter::TRACE),
				)
				.with_filter(LevelFilter::from_level(
					if settings.environment == RunningEnvironment::Development {
						Level::TRACE
					} else {
						Level::DEBUG
					},
				)),
		),
	))
	.expect("Failed to set global default subscriber");

	let authorization =
		BearerToken::from_str(&settings.api_token).expect("Failed to parse Bearer token");
	let user_agent = UserAgent::from_str(&format!("docker-runner/{}", utils::VERSION))
		.expect("Failed to parse User-Agent");

	let docker =
		bollard::Docker::connect_with_defaults().expect("Failed to connect to Docker engine");

	client::stream_request(
		ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
			.path(StreamRunnerDataForWorkspacePath {
				workspace_id: settings.workspace_id,
				runner_id: settings.runner_id,
			})
			.headers(StreamRunnerDataForWorkspaceRequestHeaders {
				authorization,
				user_agent,
			})
			.query(())
			.body(WebSocketUpgrade(PhantomData))
			.build(),
	)
	.await
	.map_err(|err| err.body)
	.expect("Cannot stream request")
	.for_each_concurrent(4, |response| async {
		use StreamRunnerDataForWorkspaceServerMsg::*;
		match response {
			DeploymentCreated {
				deployment,
				running_details,
			} => {
				if let Err(err) = docker::deployment::created(
					&settings,
					docker.clone(),
					deployment,
					running_details,
				)
				.await
				{
					error!("Failed to create deployment: {}", err);
				}
			}
			DeploymentUpdated {
				deployment,
				running_details,
			} => todo!(),
			DeploymentDeleted { id } => {
				if let Err(err) = docker::deployment::deleted(&settings, docker.clone(), id).await {
					error!("Failed to delete deployment: {}", err);
				}
			}
			Pong => (),
		}
	})
	.await;
}
