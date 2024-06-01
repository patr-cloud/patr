use models::api::user::ChangePasswordResponse;

use crate::prelude::*;

#[server(ChangePasswordFn, endpoint = "/user/change-password")]
async fn change_password(
	access_token: Option<String>,
	mfa_otp: Option<String>,
	new_password: String,
	current_password: String,
) -> Result<ChangePasswordResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::api::user::{
		ChangePasswordPath,
		ChangePasswordRequest,
		ChangePasswordRequestHeaders,
	};

	let api_response = make_api_call::<ChangePasswordRequest>(
		ApiRequest::builder()
			.path(ChangePasswordPath)
			.query(())
			.headers(ChangePasswordRequestHeaders {
				authorization: BearerToken::from_str(
					format!("Bearer {}", access_token.unwrap_or_default()).as_str(),
				)
				.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ChangePasswordRequest {
				current_password,
				new_password,
				mfa_otp,
			})
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(ServerFnError::WrappedServerError)
}
