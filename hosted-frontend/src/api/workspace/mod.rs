use models::api::user::*;

use crate::prelude::*;

mod create_workspace;
mod get_workspace_info;

pub use self::{create_workspace::*, get_workspace_info::*};

#[server(ListUserWorkspace, endpoint = "workspace")]
async fn list_user_workspace(
	access_token: Option<String>,
) -> Result<ListUserWorkspacesResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::prelude::*;

	let api_response = make_api_call::<ListUserWorkspacesRequest>(
		ApiRequest::builder()
			.path(ListUserWorkspacesPath)
			.query(())
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: BearerToken::from_str(
					format!("Bearer {}", access_token.unwrap()).as_str(),
				)
				.map_err(|e| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ListUserWorkspacesRequest {})
			.build(),
	)
	.await;

	let response_body = api_response
		.map(|res| res.body)
		.map_err(|e| ServerFnError::WrappedServerError(e));

	response_body
}
