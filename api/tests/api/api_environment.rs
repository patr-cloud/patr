use models::{ApiSuccessResponseBody, api::*};

use crate::prelude::*;

#[tokio::test]
async fn get_api_environment_reports_cloud() {
	let setup = setup().await.expect("failed to setup test server");

	// `/info` is unauthenticated — no token needed.
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetApiEnvironmentRequest>::builder()
				.headers(GetApiEnvironmentRequestHeaders {
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetApiEnvironmentResponse>>();

	assert!(
		!response.response.version.is_empty(),
		"version should be reported"
	);
	assert_eq!(
		response.response.deployment_type,
		DeploymentType::Cloud,
		"the test suite builds with the cloud feature"
	);
	assert!(
		response.response.base_domain.is_none(),
		"cloud omits base_domain; only self-hosted emits it"
	);
}
