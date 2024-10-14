use models::api::workspace::runner::*;

use crate::prelude::*;

#[server(CreateRunnerFn, endpoint = "/infrastructure/runner/create")]
pub async fn create_runner(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	name: String,
) -> Result<AddRunnerToWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<AddRunnerToWorkspaceRequest>(
		ApiRequest::builder()
			.path(AddRunnerToWorkspacePath { workspace_id })
			.query(())
			.headers(AddRunnerToWorkspaceRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(AddRunnerToWorkspaceRequest { name })
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
