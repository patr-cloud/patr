use std::{collections::HashMap, time::Duration};

use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use rand::RngExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio::{
	io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
	net::{TcpListener, TcpStream},
};

use super::{OAuthSession, make_form_request};
use crate::prelude::*;

/// The CLI's client id in the API's `oauth.clients`. A public client, since a
/// secret shipped in every copy of the binary wouldn't be one.
const CLIENT_ID: &str = "patr-cli";

/// Where the browser comes back to. The port isn't registered, since a
/// loopback redirect matches on any port (RFC 8252 section 7.3).
const CALLBACK_PATH: &str = "/callback";

/// How long to wait for the browser before giving up.
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

/// A token pair from `/token`.
pub struct Tokens {
	/// The access token.
	pub access_token: String,
	/// What renews it.
	pub session: OAuthSession,
}

/// How a refresh went, when the server decided.
pub enum RefreshOutcome {
	/// A new pair was issued.
	Refreshed(Tokens),
	/// The grant is gone: revoked, expired or replayed.
	Rejected,
}

/// `/token`'s success body, as far as the CLI reads it.
#[derive(Debug, Deserialize)]
struct TokenResponse {
	/// The access token.
	access_token: String,
	/// Always sent, since the CLI asks for `offline_access`.
	refresh_token: String,
	/// Seconds until the access token expires.
	expires_in: i64,
}

/// An OAuth error body (RFC 6749 section 5.2).
#[derive(Debug, Deserialize)]
struct TokenError {
	/// The error code.
	error: String,
	/// What went wrong, if the server said.
	error_description: Option<String>,
}

/// What a request to the loopback server turned out to be.
#[derive(Debug, PartialEq, Eq)]
pub enum CallbackOutcome {
	/// Not the callback. Browsers also ask for things like `/favicon.ico`, so
	/// the server keeps waiting.
	NotForUs,
	/// The browser came back with a code.
	Code(String),
	/// The login failed, with the reason to show the user.
	Failed(String),
}

/// Logs the user in through their browser, with the loopback flow of RFC
/// 8252. PKCE binds the code to this process.
pub async fn login() -> Result<Tokens, AppError> {
	let io_error =
		|err: std::io::Error| AppError::OAuthError(format!("the login callback failed: {err}"));

	let listener = TcpListener::bind("127.0.0.1:0").await.map_err(io_error)?;
	let redirect_uri = format!(
		"http://127.0.0.1:{}{CALLBACK_PATH}",
		listener.local_addr().map_err(io_error)?.port()
	);

	let verifier = random_token();
	let csrf_state = random_token();

	let mut authorize_url =
		url::Url::parse(&format!("{}/auth/oauth/authorize", constants::API_BASE_URL))
			.expect("the API base URL should be a URL");
	authorize_url.query_pairs_mut().extend_pairs([
		("response_type", "code"),
		("client_id", CLIENT_ID),
		("redirect_uri", &redirect_uri),
		("scope", "openid profile email offline_access"),
		("state", &csrf_state),
		(
			"code_challenge",
			&BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(&verifier)),
		),
		("code_challenge_method", "S256"),
	]);

	// Printed even when the browser opens, since over SSH it won't.
	eprintln!("Opening your browser to log in. If it doesn't open, visit:\n\n  {authorize_url}\n");
	_ = open::that(authorize_url.as_str());

	let code = tokio::time::timeout(CALLBACK_TIMEOUT, async {
		loop {
			let (mut stream, _) = listener.accept().await.map_err(io_error)?;

			let mut request_line = String::new();
			BufReader::new(&mut stream)
				.read_line(&mut request_line)
				.await
				.map_err(io_error)?;

			match interpret_callback(&request_line, &csrf_state) {
				CallbackOutcome::NotForUs => {
					respond(&mut stream, "404 Not Found", "Not found.").await;
				}
				CallbackOutcome::Failed(reason) => {
					respond(&mut stream, "400 Bad Request", &reason).await;
					return Err(AppError::OAuthError(reason));
				}
				CallbackOutcome::Code(code) => {
					respond(
						&mut stream,
						"200 OK",
						"You're logged in. You can close this tab.",
					)
					.await;
					return Ok(code);
				}
			}
		}
	})
	.await
	.map_err(|_| AppError::OAuthError("timed out waiting for the browser".to_owned()))??;

	post_token(&[
		("grant_type", "authorization_code"),
		("client_id", CLIENT_ID),
		("code", &code),
		("redirect_uri", &redirect_uri),
		("code_verifier", &verifier),
	])
	.await?
	.map_err(|error| AppError::OAuthError(error.error_description.unwrap_or(error.error)))
}

/// Reads the request line the loopback server got. A callback whose `state`
/// isn't this login's is refused, so another login's code is never
/// exchanged.
pub fn interpret_callback(request_line: &str, csrf_state: &str) -> CallbackOutcome {
	let Some(url) = request_line
		.split_whitespace()
		.nth(1)
		.and_then(|target| url::Url::parse(&format!("http://127.0.0.1{target}")).ok())
		.filter(|url| url.path() == CALLBACK_PATH)
	else {
		return CallbackOutcome::NotForUs;
	};

	let params = url.query_pairs().into_owned().collect::<HashMap<_, _>>();

	if params.get("state").map(String::as_str) != Some(csrf_state) {
		return CallbackOutcome::Failed(
			"this login didn't start here, so run `patr login` again".to_owned(),
		);
	}

	if let Some(error) = params.get("error") {
		return CallbackOutcome::Failed(params.get("error_description").unwrap_or(error).clone());
	}

	params.get("code").map_or_else(
		|| CallbackOutcome::Failed("the browser came back without a code".to_owned()),
		|code| CallbackOutcome::Code(code.clone()),
	)
}

/// Answers the browser with a one-line page. Best-effort, since the login
/// doesn't depend on the tab rendering.
async fn respond(stream: &mut TcpStream, status: &str, message: &str) {
	// The message can be an `error_description` from whoever sent the browser
	// here.
	let message = message
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;");
	let body = format!("<!doctype html><meta charset=utf-8><title>Patr</title><p>{message}</p>");

	_ = stream
		.write_all(
			format!(
				"HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: \
				 {}\r\nConnection: close\r\n\r\n{body}",
				body.len()
			)
			.as_bytes(),
		)
		.await;
}

/// Rotates a refresh token. An `Err` means the server couldn't be reached or
/// didn't decide, which isn't a dead grant.
pub async fn refresh(refresh_token: &str) -> Result<RefreshOutcome, AppError> {
	match post_token(&[
		("grant_type", "refresh_token"),
		("client_id", CLIENT_ID),
		("refresh_token", refresh_token),
	])
	.await?
	{
		Ok(tokens) => Ok(RefreshOutcome::Refreshed(tokens)),
		Err(error)
			if matches!(
				error.error.as_str(),
				"invalid_grant" | "invalid_client" | "unauthorized_client"
			) =>
		{
			Ok(RefreshOutcome::Rejected)
		}
		Err(error) => Err(AppError::OAuthError(
			error.error_description.unwrap_or(error.error),
		)),
	}
}

/// Revokes the grant behind a refresh token. Best-effort, so logging out still
/// works offline.
pub async fn revoke(refresh_token: &str) {
	if let Err(err) = make_form_request(
		"/auth/oauth/revoke",
		&[("token", refresh_token), ("client_id", CLIENT_ID)],
	)
	.await
	.and_then(reqwest::Response::error_for_status)
	{
		eprintln!(
			"Couldn't revoke this login ({err}). Revoke it from Authorized Apps in the dashboard."
		);
	}
}

/// POSTs to `/token`. The outer `Result` is the request, the inner one the
/// server's answer.
async fn post_token(form: &[(&str, &str)]) -> Result<Result<Tokens, TokenError>, AppError> {
	let response = make_form_request("/auth/oauth/token", form).await?;
	let status = response.status();
	let body = response.text().await?;

	if !status.is_success() {
		return Ok(Err(serde_json::from_str(&body).unwrap_or_else(|_| {
			TokenError {
				error: format!("the token endpoint returned {status}"),
				error_description: None,
			}
		})));
	}

	let response = serde_json::from_str::<TokenResponse>(&body)
		.map_err(|err| AppError::OAuthError(format!("couldn't read the token response: {err}")))?;

	Ok(Ok(Tokens {
		access_token: response.access_token,
		session: OAuthSession {
			refresh_token: response.refresh_token,
			token_expiry: OffsetDateTime::now_utc().unix_timestamp() + response.expires_in,
		},
	}))
}

/// 32 random bytes, base64url-encoded: 43 characters, the shortest PKCE
/// verifier RFC 7636 allows.
fn random_token() -> String {
	BASE64_URL_SAFE_NO_PAD.encode(rand::rng().random::<[u8; 32]>())
}
