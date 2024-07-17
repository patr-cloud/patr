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
		let access_cookie =
			Cookie::build((constants::ACCESS_TOKEN, resp.body.access_token.clone()))
				.path("/")
				.max_age(Duration::days(90))
				.same_site(SameSite::Lax)
				.build();
		let refresh_cookie =
			Cookie::build((constants::REFRESH_TOKEN, resp.body.refresh_token.clone()))
				.path("/")
				.max_age(Duration::days(90))
				.same_site(SameSite::Lax)
				.build();
		let access_token_header = HeaderValue::from_str(access_cookie.to_string().as_str());
		let refresh_token_header = HeaderValue::from_str(refresh_cookie.to_string().as_str());

		if let (Ok(access_token_header), Ok(refresh_token_header)) =
			(access_token_header, refresh_token_header)
		{
			response.append_header(SET_COOKIE, access_token_header);
			response.append_header(SET_COOKIE, refresh_token_header);
			leptos_axum::redirect("/");
		}
	}

	Ok(api_response.map(|res| res.body)?)
}
