use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(StartDeploymentFn, endpoint = "/infrastructure/deployment/start")]
pub async fn start_deployment(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	deployment_id: Uuid,
) -> Result<StartDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = access_token
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;
	let access_token = BearerToken::from_str(access_token.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<StartDeploymentRequest>(
		ApiRequest::builder()
			.path(StartDeploymentPath {
				deployment_id,
				workspace_id,
			})
			.query(())
			.headers(StartDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(StartDeploymentRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
