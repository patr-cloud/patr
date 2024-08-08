use models::api::workspace::rbac::*;

use crate::prelude::*;

#[server(ListAppPermissionsFn, endpoint = "/workspace/rbac/permissions")]
pub async fn list_all_permissions(
	access_token: Option<String>,
	workspace_id: Option<String>,
) -> Result<ListAllPermissionsResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::api::workspace::rbac::*;

	use crate::prelude::*;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	// let api_response = make_api_call::<ListDeploymentRequest>(
	let api_response = make_api_call::<ListAllPermissionsRequest>(
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
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
