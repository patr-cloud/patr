use models::api::workspace::runner::*;

use crate::prelude::*;

#[server(ListRunnersFn, endpoint = "/infrastructure/runner/list")]
pub async fn list_runners(
	access_token: Option<String>,
	workspace_id: Uuid,
) -> Result<ListRunnersForWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<ListRunnersForWorkspaceRequest>(
		ApiRequest::builder()
			.path(ListRunnersForWorkspacePath { workspace_id })
			.query(Paginated {
				data: (),
				page: 0,
				count: 10,
			})
			.headers(ListRunnersForWorkspaceRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(ListRunnersForWorkspaceRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
