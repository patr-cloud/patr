use std::{marker::PhantomData, str::FromStr};

use bollard::{
	container::{Config, CreateContainerOptions, StartContainerOptions},
	secret::HealthConfig,
};
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

/// The prelude module contains all the commonly used types and traits that are
/// used across the crate. This is mostly used to avoid having to import a lot
/// of things from different modules.
pub mod prelude {
	pub use models::prelude::*;
	pub use tracing::{debug, error, info, instrument, trace, warn};
}

#[tokio::main]
async fn main() {
	let RunnerSettings {
		workspace_id,
		runner_id,
		api_token,
		environment,
	} = config::get_runner_settings();

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
					if environment == RunningEnvironment::Development {
						Level::TRACE
					} else {
						Level::DEBUG
					},
				)),
		),
	))
	.expect("Failed to set global default subscriber");

	let authorization = BearerToken::from_str(&api_token).expect("Failed to parse Bearer token");
	let user_agent = UserAgent::from_static("runner/docker");

	client::stream_request(
		ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
			.path(StreamRunnerDataForWorkspacePath {
				workspace_id,
				runner_id,
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
	.for_each_concurrent(4, |response| async move {
		use StreamRunnerDataForWorkspaceServerMsg::*;
		match response {
			DeploymentCreated(deployment) => {
				bollard::Docker::connect_with_local_defaults()
					.expect("Failed to connect to Docker")
					.create_container(
						Some(CreateContainerOptions {
							name: deployment.data.name.clone(),
							..Default::default()
						}),
						Config {
							image: Some(format!(
								"{}/{}:{}",
								deployment.data.registry.registry_url(),
								deployment.data.registry.image_name().unwrap(),
								deployment.data.image_tag
							)),
							..Default::default()
						},
					)
					.await
					.expect("Failed to create container");
				bollard::Docker::connect_with_local_defaults()
					.expect("Failed to connect to Docker")
					.start_container(&deployment.data.name, None::<StartContainerOptions<String>>)
					.await
					.expect("Failed to start container");
			}
			DeploymentUpdated { id, old, new } => todo!(),
			DeploymentDeleted(_) => todo!(),
			Pong => (),
		}
	})
	.await;
}
