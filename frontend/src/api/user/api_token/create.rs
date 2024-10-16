use models::api::user::*;

use crate::prelude::*;

#[server(CreateApiTokenFn, endpoint = "/user/api-token/create")]
pub async fn create_api_token(
	access_token: Option<String>,
	api_token_info: CreateApiTokenRequest,
) -> Result<CreateApiTokenResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	logging::log!("here");
	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<CreateApiTokenRequest>(
		ApiRequest::builder()
			.path(CreateApiTokenPath)
			.query(())
			.headers(CreateApiTokenRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(api_token_info)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
