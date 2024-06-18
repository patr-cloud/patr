use models::api::user::*;

use crate::prelude::*;

#[server(GetApiTokenFn, endpoint = "/user/api-token/get")]
pub async fn get_api_token(
	access_token: Option<String>,
	token_id: String,
) -> Result<GetApiTokenInfoResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let token_id = Uuid::parse_str(token_id.clone().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = make_api_call::<GetApiTokenInfoRequest>(
		ApiRequest::builder()
			.path(GetApiTokenInfoPath { token_id })
			.query(())
			.headers(GetApiTokenInfoRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(GetApiTokenInfoRequest)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
