use axum::http::StatusCode;
use chrono::{Duration, Utc};
use cookie::Cookie;
use jsonwebtoken::{EncodingKey, Header, encode};
use models::api::auth::oauth::*;
use rand::{Rng, distributions::Alphanumeric};

use crate::prelude::*;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AuthClaims {
	pub auth_code: String,
	pub code_challenge: String,
	pub exp: usize,
}

/// Generate a random authorization code
pub fn generate_authorization_code() -> String {
	rand::thread_rng()
		.sample_iter(&Alphanumeric)
		.take(32)
		.map(char::from)
		.collect()
}

pub fn create_token(code_challenge: String, secret: &str) -> String {
	let auth_code: String = generate_authorization_code();
	info!("Generated auth code: {}", auth_code);
	let claims = AuthClaims {
		auth_code,
		code_challenge,
		exp: (Utc::now() + Duration::minutes(10)).timestamp() as usize,
	};
	encode(
		&Header::default(),
		&claims,
		&EncodingKey::from_secret(secret.as_ref()),
	)
	.expect("Failed to encode JWT")
}

pub async fn authorize(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthAuthorizePath,
				query:
					OAuthAuthorizeQuery {
						response_type,
						client_id,
						redirect_uri,
						scope,
						state,
						code_challenge,
						code_challenge_method,
					},
				headers: OAuthAuthorizeRequestHeaders { cookie },
				body: OAuthAuthorizeRequestProcessed,
			},
		database,
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, OAuthAuthorizeRequest>,
) -> Result<AppResponse<OAuthAuthorizeRequest>, ErrorType> {
	if response_type != OAuthAuthorizeResponseType::AuthorizationCode {
		return Err(ErrorType::InternalServerError);
	}

	// Check if user is logged in by checking the session_user cookie
	if cookie.is_some() {
		if let Some(session_token) = cookie.get("session_user") {
			tracing::info!("Found session_user cookie with value: {}", session_token);

			// Create authorization code
			let auth_code = create_token(code_challenge.clone(), &config.jwt_secret);

			// Set the auth_token cookie
			let cookie = Cookie::build(("auth_token", auth_code.clone()))
				.path(redirect_uri.as_deref().unwrap_or("/"))
				.http_only(true);

			// Build redirect with code + state
			let mut redirect_url =
				format!("{}?code={}", redirect_uri.unwrap_or_default(), auth_code);

			if let Some(state) = state {
				redirect_url.push_str(&format!("&state={}", state));
			}

			let redirect_url = redirect_url.parse().expect("a valid redirect url");
			return AppResponse::builder()
				.body(OAuthAuthorizeResponse)
				.headers(OAuthAuthorizeResponseHeaders { redirect_url })
				.status_code(StatusCode::FOUND)
				.build()
				.into_result();
		} else {
			tracing::info!("session_user cookie not found in cookie header");
		}
	} else {
		tracing::info!("No Cookie header present in request");
	}

	// User not logged in, redirect to login page
	let login = "http://localhost:3001/auth/sign-in"
		.parse()
		.expect("cannot find login url");

	AppResponse::builder()
		.body(OAuthAuthorizeResponse)
		.headers(OAuthAuthorizeResponseHeaders {
			redirect_url: login,
		})
		.status_code(StatusCode::FOUND)
		.build()
		.into_result()
}
