use std::{marker::PhantomData, str::FromStr};

use config::RunnerSettings;
use futures::StreamExt;
use models::{api::workspace::runner::*, prelude::*, utils::WebSocketUpgrade};

/// The client for the Patr API to get runner data for a given workspace.
mod client;
/// The configuration for the runner.
mod config;

/// The prelude module contains all the commonly used types and traits that are
/// used across the crate. This is mostly used to avoid having to import a lot
/// of things from different modules.
pub mod prelude {
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
	.map_err(|err| err.body.message)
	.expect("Cannot stream request")
	.for_each_concurrent(4, |response| async move {
		println!("{:#?}", response);
	})
	.await;
}