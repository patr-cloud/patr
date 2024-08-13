use models::api::workspace::runner::*;

use crate::prelude::*;

#[server(DeleteRunnerFn, endpoint = "/infrastructure/runner/delete")]
pub async fn delete_runner(
	runner_id: Option<String>,
	access_token: Option<String>,
	workspace_id: Option<String>,
) -> Result<DeleteRunnerResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let runner_id = Uuid::parse_str(runner_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<DeleteRunnerRequest>(
		ApiRequest::builder()
			.path(DeleteRunnerPath {
				workspace_id,
				runner_id,
			})
			.query(())
			.headers(DeleteRunnerRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(DeleteRunnerRequest)
			.build(),
	)
	.await;

	if api_response.is_ok() {
		leptos_axum::redirect("/runners");
	}

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
