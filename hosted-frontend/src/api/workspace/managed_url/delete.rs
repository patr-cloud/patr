use models::api::workspace::managed_url::*;

use crate::prelude::*;

#[server(DeleteManagedURLFn, endpoint = "/domain-config/managed-url/delete")]
pub async fn delete_managed_url(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	managed_url_id: Option<String>,
) -> Result<DeleteManagedURLResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(&workspace_id.unwrap().to_string())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let managed_url_id = Uuid::parse_str(managed_url_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<DeleteManagedURLRequest>(
		ApiRequest::builder()
			.path(DeleteManagedURLPath {
				managed_url_id,
				workspace_id,
			})
			.query(())
			.headers(DeleteManagedURLRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(DeleteManagedURLRequest)
			.build(),
	)
	.await
	.map(|res| {
		leptos_axum::redirect("/managed-url");
		res.body
	})
	.map_err(ServerFnError::WrappedServerError)
}
