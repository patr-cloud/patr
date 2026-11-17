//! Tests for the `patr deployment` commands.

pub mod list;
pub mod start;
pub mod stop;

use cli::prelude::*;
use models::api::workspace::runner::*;
use wiremock::{
	Mock,
	MockServer,
	ResponseTemplate,
	matchers::{method, path},
};

use crate::{
	apply::{external_deployment, mount_deployment_list},
	setup,
};

/// The deployment `start` and `stop` act on.
const DEPLOYMENT_NAME: &str = "my-app";
/// The runner [`DEPLOYMENT_NAME`] runs on.
const RUNNER_NAME: &str = "prod-1";

/// The IDs `start` and `stop` tests use.
struct Ids {
	workspace: Uuid,
	deployment: Uuid,
	runner: Uuid,
	machine_type: Uuid,
}

impl Ids {
	fn new() -> Self {
		Self {
			workspace: Uuid::parse_str("00000000000000000000000000000001").unwrap(),
			deployment: Uuid::parse_str("00000000000000000000000000000002").unwrap(),
			runner: Uuid::parse_str("00000000000000000000000000000003").unwrap(),
			machine_type: Uuid::parse_str("00000000000000000000000000000004").unwrap(),
		}
	}
}

/// Mount the deployment list `start` and `stop` resolve [`DEPLOYMENT_NAME`]
/// from, and answer the runner lookup they make before acting with
/// `runner_info`.
async fn mount_lookups(server: &MockServer, ids: &Ids, runner_info: ResponseTemplate) {
	mount_deployment_list(
		server,
		ids.workspace,
		vec![WithId::new(
			ids.deployment,
			external_deployment(DEPLOYMENT_NAME, ids.runner, ids.machine_type, "1.27"),
		)],
	)
	.await;

	Mock::given(method("GET"))
		.and(path(format!(
			"/workspace/{}/runner/{}",
			ids.workspace, ids.runner
		)))
		.respond_with(runner_info)
		.mount(server)
		.await;
}

/// A `GetRunnerInfo` response for [`RUNNER_NAME`], `connected` or not.
fn runner_info(connected: bool) -> ResponseTemplate {
	setup::success(GetRunnerInfoResponse {
		runner: WithId::new(
			Ids::new().runner,
			Runner {
				name: RUNNER_NAME.to_string(),
				connected,
				last_seen: None,
				version: "0.1.0".parse().expect("valid semver"),
			},
		),
	})
}
