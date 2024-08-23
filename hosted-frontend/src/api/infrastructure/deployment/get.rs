use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(GetDeploymentFn, endpoint = "/infrastructure/deployment/get")]
pub async fn get_deployment(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	deployment_id: Uuid,
) -> Result<GetDeploymentInfoResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = access_token
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;
	let access_token = BearerToken::from_str(access_token.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<GetDeploymentInfoRequest>(
		ApiRequest::builder()
			.path(GetDeploymentInfoPath {
				deployment_id,
				workspace_id,
			})
			.query(())
			.headers(GetDeploymentInfoRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(GetDeploymentInfoRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
