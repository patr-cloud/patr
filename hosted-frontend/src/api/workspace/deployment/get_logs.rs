use models::api::workspace::deployment::*;
use time::OffsetDateTime;

use crate::prelude::*;

#[server(GetDeploymentLogsFn, endpoint = "/infrastructure/deployment/get_logs")]
pub async fn get_deployment_logs(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	deployment_id: Uuid,
	end_time: Option<OffsetDateTime>,
	limit: Option<u32>,
) -> Result<GetDeploymentLogsResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = access_token
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;
	let access_token = BearerToken::from_str(access_token.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<GetDeploymentLogsRequest>(
		ApiRequest::builder()
			.path(GetDeploymentLogsPath {
				deployment_id,
				workspace_id,
			})
			.query(GetDeploymentLogsQuery { end_time, limit })
			.headers(GetDeploymentLogsRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(GetDeploymentLogsRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
