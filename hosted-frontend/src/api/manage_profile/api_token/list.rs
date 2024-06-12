use models::api::user::ListApiTokensResponse;

use crate::prelude::*;

#[server(LoadApiTokenFn, endpoint = "/user/api-token")]
pub async fn load_api_tokens_list(
	access_token: Option<String>,
) -> Result<ListApiTokensResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::api::user::{ListApiTokensPath, ListApiTokensRequest, ListApiTokensRequestHeaders};

	let api_response = make_api_call::<ListApiTokensRequest>(
		ApiRequest::builder()
			.path(ListApiTokensPath)
			.query(Paginated {
				data: (),
				page: 0,
				count: 10,
			})
			.headers(ListApiTokensRequestHeaders {
				authorization: BearerToken::from_str(access_token.unwrap().as_str()).map_err(
					|e| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken),
				)?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ListApiTokensRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(ServerFnError::WrappedServerError)
}
