use models::api::workspace::managed_url::*;

use crate::prelude::*;

#[server(DeleteManagedURLFn, endpoint = "/domain-config/managed-url/delete")]
pub async fn delete_managed_url(
	access_token: Option<String>,
	workspace_id: Option<String>,
	managed_url_id: Option<String>,
) -> Result<DeleteManagedURLResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let managed_url_id = Uuid::parse_str(managed_url_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<DeleteManagedURLRequest>(
		ApiRequest::builder()
			.path(DeleteManagedURLPath {
				managed_url_id,
				workspace_id,
			})
			.query(())
			.headers(DeleteManagedURLRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(DeleteManagedURLRequest)
			.build(),
	)
	.await;

	if api_response.is_ok() {
		leptos_axum::redirect("/managed-url");
	}

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
