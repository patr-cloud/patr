use models::api::auth::*;

use crate::{global_state::authstate_from_cookie, prelude::*};

/// Server function for completing the sign up process
#[server(ConfirmOtp, endpoint = "auth/join")]
async fn complete_sign_up(
	username: String,
	otp: String,
) -> Result<CompleteSignUpResponse, ServerFnError<ErrorType>> {
	use axum::http::header::{HeaderValue, SET_COOKIE};
	use axum_extra::extract::cookie::{Cookie, SameSite};
	use leptos_axum::ResponseOptions;
	// use axum_extra::extract::cookie::Cookie;
	use models::api::auth::{
		CompleteSignUpPath,
		CompleteSignUpRequest,
		CompleteSignUpRequestHeaders,
	};
	use time::Duration;

	let api_response = make_api_call::<CompleteSignUpRequest>(
		ApiRequest::builder()
			.path(CompleteSignUpPath)
			.query(())
			.headers(CompleteSignUpRequestHeaders {
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CompleteSignUpRequest {
				username,
				verification_token: otp,
			})
			.build(),
	)
	.await;

	let response = expect_context::<ResponseOptions>();

	if let Ok(resp) = &api_response {
		logging::log!("{:#?}", resp.body);
		let access_cookie = Cookie::build(("access_token", resp.body.access_token.clone()))
			.path("/")
			.max_age(Duration::days(90))
			.same_site(SameSite::Lax)
			.build();
		let refresh_cookie = Cookie::build(("refresh_token", resp.body.refresh_token.clone()))
			.path("/")
			.max_age(Duration::days(90))
			.same_site(SameSite::Lax)
			.build();
		let access_token_header = HeaderValue::from_str(access_cookie.to_string().as_str());
		let refresh_token_header = HeaderValue::from_str(refresh_cookie.to_string().as_str());

		if let (Ok(access_token_cookie), Ok(refresh_token_cookie)) =
			(access_token_header, refresh_token_header)
		{
			response.append_header(SET_COOKIE, access_token_cookie);
			response.append_header(SET_COOKIE, refresh_token_cookie);
			leptos_axum::redirect("/");
		}
	}

	Ok(api_response.map(|res| res.body)?)
}
