use models::api::workspace::rbac::*;

use crate::prelude::*;

#[server(ListAppPermissionsFn, endpoint = "/workspace/rbac/permissions")]
pub async fn list_all_permissions(
	access_token: Option<String>,
	workspace_id: Uuid,
) -> Result<ListAllPermissionsResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<ListAllPermissionsRequest>(
		ApiRequest::builder()
			.path(ListAllPermissionsPath { workspace_id })
			.query(())
			.headers(ListAllPermissionsRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ListAllPermissionsRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
