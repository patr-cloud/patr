use models::api::user::*;

use crate::prelude::*;

#[server(GetApiTokenFn, endpoint = "/user/api-token/get")]
pub async fn get_api_token(
	access_token: Option<String>,
	token_id: Uuid,
) -> Result<GetApiTokenInfoResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	make_api_call::<GetApiTokenInfoRequest>(
		ApiRequest::builder()
			.path(GetApiTokenInfoPath { token_id })
			.query(())
			.headers(GetApiTokenInfoRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(GetApiTokenInfoRequest)
			.build(),
	)
	.await
	.map(|res| res.body)
	.map_err(ServerFnError::WrappedServerError)
}
