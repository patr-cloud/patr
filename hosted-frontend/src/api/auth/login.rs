use models::api::auth::*;

use crate::prelude::*;

/// The API endpoint for logging in to the application. This endpoint is used to
/// authenticate the user and get the JWT tokens for the user.
#[server(LoginFn, endpoint = "auth/sign-in")]
pub async fn login(
	user_id: String,
	password: String,
	mfa_otp: Option<String>,
) -> Result<LoginResponse, ServerFnError<ErrorType>> {
	use axum::http::header::{HeaderValue, SET_COOKIE};
	use axum_extra::extract::cookie::{Cookie, SameSite};
	use leptos_axum::ResponseOptions;
	use models::api::auth::*;
	use time::Duration;

	let api_response = make_api_call::<LoginRequest>(
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
	.await;

	let response = expect_context::<ResponseOptions>();

	if let Ok(ref resp) = api_response {
		AuthState::load().1.set(Some(AuthState::LoggedIn {
			access_token: resp.body.access_token.clone(),
			refresh_token: resp.body.refresh_token.clone(),
			last_used_workspace_id: None,
		}));
		leptos_axum::redirect("/");
	}

	Ok(api_response.map(|res| res.body)?)
}
