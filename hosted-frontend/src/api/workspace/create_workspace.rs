use models::api::workspace::*;

use crate::prelude::*;

#[server(CreateWorkspaceFn, endpoint = "/workspace/create")]
pub async fn create_workspace(
	access_token: Option<String>,
	workspace_name: String,
) -> Result<CreateWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::api::workspace::{
		CreateWorkspacePath,
		CreateWorkspaceRequest,
		CreateWorkspaceRequestHeaders,
	};

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let api_response = make_api_call::<CreateWorkspaceRequest>(
		ApiRequest::builder()
			.path(CreateWorkspacePath)
			.query(())
			.headers(CreateWorkspaceRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CreateWorkspaceRequest {
				name: workspace_name,
			})
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
