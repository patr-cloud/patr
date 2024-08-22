use models::api::workspace::runner::*;

use crate::prelude::*;

#[server(ListRunnersFn, endpoint = "/infrastructure/runner/list")]
pub async fn list_runners(
	workspace_id: Option<String>,
	access_token: Option<String>,
) -> Result<ListRunnersForWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<ListRunnersForWorkspaceRequest>(
		ApiRequest::builder()
			.path(ListRunnersForWorkspacePath { workspace_id })
			.query(Paginated {
				data: (),
				page: 0,
				count: 10,
			})
			.headers(ListRunnersForWorkspaceRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(ListRunnersForWorkspaceRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
