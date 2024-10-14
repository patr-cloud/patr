use models::api::user::*;

use crate::prelude::*;

#[server(RegenerateApiTokenFn, endpoint = "/user/api-token/regenerate")]
pub async fn regenerate_api_token(
	access_token: Option<String>,
	token_id: String,
) -> Result<RegenerateApiTokenResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let token_id = Uuid::parse_str(token_id.clone().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	make_api_call::<RegenerateApiTokenRequest>(
		ApiRequest::builder()
			.path(RegenerateApiTokenPath { token_id })
			.query(())
			.headers(RegenerateApiTokenRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_str("hyper/0.12.2").unwrap(),
			})
			.body(RegenerateApiTokenRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
