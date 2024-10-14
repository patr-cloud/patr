use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(StartDeploymentFn, endpoint = "/infrastructure/deployment/start")]
pub async fn start_deployment(
	access_token: Option<String>,
	workspace_id: Uuid,
	deployment_id: Uuid,
) -> Result<StartDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = access_token
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;
	let access_token = BearerToken::from_str(access_token.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<StartDeploymentRequest>(
		ApiRequest::builder()
			.path(StartDeploymentPath {
				deployment_id,
				workspace_id,
			})
			.query(StartDeploymentQuery {
				force_restart: false,
			})
			.headers(StartDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(StartDeploymentRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
