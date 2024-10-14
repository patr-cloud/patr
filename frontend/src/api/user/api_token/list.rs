use models::api::user::*;

use crate::prelude::*;

#[server(LoadApiTokenFn, endpoint = "/user/api-token")]
pub async fn load_api_tokens_list(
	access_token: Option<String>,
) -> Result<ListApiTokensResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	make_api_call::<ListApiTokensRequest>(
		ApiRequest::builder()
			.path(ListApiTokensPath)
			.query(Paginated {
				data: (),
				page: 0,
				count: 10,
			})
			.headers(ListApiTokensRequestHeaders {
				authorization: BearerToken::from_str(access_token.unwrap().as_str()).map_err(
					|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken),
				)?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ListApiTokensRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
