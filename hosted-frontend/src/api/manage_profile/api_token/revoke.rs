use models::api::{self, user::*};

use crate::prelude::*;

#[server(RevokeApiTokenFn, endpoint = "/user/api-token/delete")]
pub async fn revoke_api_token(
	access_token: Option<String>,
	token_id: String,
) -> Result<RevokeApiTokenResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let token_id = Uuid::parse_str(token_id.clone().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<RevokeApiTokenRequest>(
		ApiRequest::builder()
			.path(RevokeApiTokenPath { token_id })
			.query(())
			.headers(RevokeApiTokenRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_str("hyper/0.12.2").unwrap(),
			})
			.body(RevokeApiTokenRequest)
			.build(),
	)
	.await;

	if api_response.is_ok() {
		leptos_axum::redirect("/user/api-tokens");
	}

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}