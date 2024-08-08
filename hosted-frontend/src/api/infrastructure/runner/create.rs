use models::api::workspace::runner::*;

use crate::prelude::*;

#[server(CreateRunnerFn, endpoint = "/infrastructure/runner/create")]
pub async fn create_runner(
	name: String,
	access_token: Option<String>,
	workspace_id: Option<String>,
) -> Result<AddRunnerToWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<AddRunnerToWorkspaceRequest>(
		ApiRequest::builder()
			.path(AddRunnerToWorkspacePath { workspace_id })
			.query(())
			.headers(AddRunnerToWorkspaceRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(&USER_AGENT_STRING),
			})
			.body(AddRunnerToWorkspaceRequest { name })
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
