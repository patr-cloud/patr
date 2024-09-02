use models::api::workspace::*;

use crate::prelude::*;

#[server(GetWorkspaceInfoFn, endpoint = "/workspace/create")]
pub async fn get_workspace_info(
	access_token: Option<String>,
	workspace_id: Uuid,
) -> Result<GetWorkspaceInfoResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = access_token
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;
	let access_token = BearerToken::from_str(access_token.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let api_response = make_api_call::<GetWorkspaceInfoRequest>(
		ApiRequest::builder()
			.path(GetWorkspaceInfoPath { workspace_id })
			.query(())
			.headers(GetWorkspaceInfoRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(GetWorkspaceInfoRequest {})
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
