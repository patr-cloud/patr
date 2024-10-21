use leptos::server_fn::codec::Json;
use models::api::user::*;

use crate::prelude::*;

#[server(ListUserWorkspaceFn, endpoint = "user/workspace/list", input = Json)]
pub async fn list_user_workspace(
	access_token: String,
) -> Result<ListUserWorkspacesResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::prelude::*;
	let access_token = BearerToken::from_str(access_token.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let api_response = make_api_call::<ListUserWorkspacesRequest>(
		ApiRequest::builder()
			.path(ListUserWorkspacesPath)
			.query(())
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ListUserWorkspacesRequest {})
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
