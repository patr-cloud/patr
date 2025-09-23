use models::api::workspace::runner::*;

use crate::prelude::*;
/// List runners for a workspace
#[server(ListRunnersFn, endpoint = "/infrastructure/runner/list")]
pub async fn list_runners(
	/// The access token of the user
	access_token: Option<String>,
	/// The workspace id to list runners for
	workspace_id: Uuid,
) -> Result<ListRunnersForWorkspaceResponse, AppError> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| AppError::General(format!("Invalid token")))?;

	make_api_call::<ListRunnersForWorkspaceRequest>(
		ApiRequest::builder()
			.path(ListRunnersForWorkspacePath { workspace_id })
			.query(Default::default())
			.headers(ListRunnersForWorkspaceRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(ListRunnersForWorkspaceRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(|e| AppError::General(format!("Api failed:{e}")))
}
