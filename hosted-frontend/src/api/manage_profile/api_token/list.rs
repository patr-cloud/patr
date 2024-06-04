use models::api::user::ListApiTokensResponse;

use crate::prelude::*;

#[server(LoadApiTokenFn, endpoint = "/user/api-token")]
pub async fn load_api_tokens_list(
	access_token: Option<String>,
) -> Result<Result<ListApiTokensResponse, ErrorType>, ServerFnError> {
	use std::str::FromStr;

	use models::api::user::{ListApiTokensPath, ListApiTokensRequest, ListApiTokensRequestHeaders};

	let api_response = make_api_call::<ListApiTokensRequest>(
		ApiRequest::builder()
			.path(ListApiTokensPath)
			.query(Default::default())
			.headers(ListApiTokensRequestHeaders {
				authorization: BearerToken::from_str(
					format!("{}", access_token.unwrap_or_default()).as_str(),
				)?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ListApiTokensRequest)
			.build(),
	)
	.await;

	Ok(api_response.map(|res| res.body))
}
