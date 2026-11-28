//! `patr deployment start`.

use models::api::workspace::deployment::*;
use wiremock::{
	Mock,
	ResponseTemplate,
	matchers::{method, path},
};

use super::*;
use crate::setup;

/// Run `patr deployment start` with the runner lookup answered by
/// `runner_info`, and assert it sent `starts` start requests.
async fn start(runner_info: ResponseTemplate, starts: u64) -> Result<CommandOutput, AppError> {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_lookups(server, &ids, runner_info).await;
	Mock::given(method("POST"))
		.and(path(format!(
			"/workspace/{}/deployment/{}/start",
			ids.workspace, ids.deployment
		)))
		.respond_with(setup::success(StartDeploymentResponse))
		.expect(starts)
		.mount(server)
		.await;

	let output = setup::run(
		setup::state(ids.workspace),
		&["deployment", "start", "--name", DEPLOYMENT_NAME],
	)
	.await;

	server.verify().await;

	output
}

#[tokio::test]
async fn names_the_runner_that_will_deploy_it() {
	assert_eq!(
		start(runner_info(true), 1).await.unwrap().text,
		"Start requested. The runner \"prod-1\" will deploy \"my-app\"."
	);
}

#[tokio::test]
async fn says_when_the_runner_is_offline() {
	assert_eq!(
		start(runner_info(false), 1).await.unwrap().text,
		"Start requested. The runner \"prod-1\" is offline, so \"my-app\" will start once it \
		 reconnects."
	);
}

#[tokio::test]
async fn starts_without_runner_details_when_the_runner_is_hidden() {
	assert_eq!(
		start(setup::error(ErrorType::Unauthorized), 1)
			.await
			.unwrap()
			.text,
		"Start requested."
	);
}

#[tokio::test]
async fn does_not_start_when_the_runner_lookup_fails() {
	assert!(matches!(
		start(setup::error(ErrorType::InternalServerError), 0).await,
		Err(AppError::ApiError(ErrorType::InternalServerError))
	));
}
