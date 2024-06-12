use models::api::auth::*;

use crate::prelude::*;

/// The API endpoint for logging in to the application. This endpoint is used to
/// authenticate the user and get the JWT tokens for the user.
#[server(Login, endpoint = "auth/sign-in")]
pub async fn login(
	user_id: String,
	password: String,
	mfa_otp: Option<String>,
) -> Result<LoginResponse, ServerFnError<ErrorType>> {
	use models::api::auth::*;

	let response = make_api_call::<LoginRequest>(
		ApiRequest::builder()
			.path(LoginPath)
			.query(())
			.headers(LoginRequestHeaders {
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(LoginRequest {
				user_id,
				password,
				mfa_otp,
			})
			.build(),
	)
	.await?;

	Ok(response.body)
}
