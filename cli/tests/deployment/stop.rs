//! `patr deployment stop`.

use models::api::workspace::deployment::*;
use wiremock::{
	Mock,
	ResponseTemplate,
	matchers::{method, path},
};

use super::*;
use crate::setup;

/// Run `patr deployment stop` with the runner lookup answered by
/// `runner_info`, and assert it sent `stops` stop requests.
async fn stop(runner_info: ResponseTemplate, stops: u64) -> Result<CommandOutput, AppError> {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_lookups(server, &ids, runner_info).await;
	Mock::given(method("POST"))
		.and(path(format!(
			"/workspace/{}/deployment/{}/stop",
			ids.workspace, ids.deployment
		)))
		.respond_with(setup::success(StopDeploymentResponse))
		.expect(stops)
		.mount(server)
		.await;

	let output = setup::run(
		setup::state(ids.workspace),
		&["deployment", "stop", "--name", DEPLOYMENT_NAME],
	)
	.await;

	server.verify().await;

	output
}

#[tokio::test]
async fn names_the_runner_that_will_shut_it_down() {
	assert_eq!(
		stop(runner_info(true), 1).await.unwrap().text,
		"Stop requested. The runner \"prod-1\" will shut down \"my-app\". Its volumes are \
		 kept."
	);
}

#[tokio::test]
async fn says_when_the_runner_is_offline() {
	assert_eq!(
		stop(runner_info(false), 1).await.unwrap().text,
		"Stop requested. The runner \"prod-1\" is offline, so \"my-app\" will stop once it \
		 reconnects."
	);
}

#[tokio::test]
async fn stops_without_runner_details_when_the_runner_is_hidden() {
	assert_eq!(
		stop(setup::error(ErrorType::Unauthorized), 1)
			.await
			.unwrap()
			.text,
		"Stop requested."
	);
}

#[tokio::test]
async fn does_not_stop_when_the_runner_lookup_fails() {
	assert!(matches!(
		stop(setup::error(ErrorType::InternalServerError), 0).await,
		Err(AppError::ApiError(ErrorType::InternalServerError))
	));
}
