use models::api::workspace::runner::*;

use crate::prelude::*;

#[server(GetRunnerInfoFn, endpoint = "/infrastructure/runner/get-info")]
pub async fn get_runner(
	access_token: Option<String>,
	runner_id: Option<String>,
	workspace_id: Option<String>,
) -> Result<GetRunnerInfoResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let runner_id = Uuid::parse_str(runner_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<GetRunnerInfoRequest>(
		ApiRequest::builder()
			.path(GetRunnerInfoPath {
				workspace_id,
				runner_id,
			})
			.query(())
			.headers(GetRunnerInfoRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(&USER_AGENT_STRING),
			})
			.body(GetRunnerInfoRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
