use models::api::user::*;

use crate::prelude::*;

#[server(CreateApiTokenFn, endpoint = "/user/api-token/create")]
pub async fn create_api_token(
	access_token: Option<String>,
	api_token_info: CreateApiTokenRequest,
) -> Result<CreateApiTokenResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::api::user::*;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let api_response = make_api_call::<CreateApiTokenRequest>(
		ApiRequest::builder()
			.path(CreateApiTokenPath)
			.query(())
			.headers(CreateApiTokenRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(constants::USER_AGENT_STRING),
			})
			.body(api_token_info)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
