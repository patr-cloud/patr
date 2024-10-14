use models::api::workspace::managed_url::*;

use crate::prelude::*;

#[server(ListManagedURLs, endpoint = "/domain-config/managed-url/list")]
pub async fn list_managed_urls(
	workspace_id: Option<Uuid>,
	access_token: Option<String>,
) -> Result<ListManagedURLResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(&workspace_id.unwrap().to_string())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<ListManagedURLRequest>(
		ApiRequest::builder()
			.path(ListManagedURLPath { workspace_id })
			.query(Paginated {
				count: 10,
				page: 0,
				data: ListManagedURLQuery {
					order: None,
					order_by: None,
					filter: None,
				},
			})
			.headers(ListManagedURLRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(ListManagedURLRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
