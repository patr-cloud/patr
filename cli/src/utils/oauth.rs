use std::{
	collections::{BTreeMap, HashMap},
	time::Duration,
};

use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use rand::RngExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio::{
	io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
	net::TcpListener,
};

use crate::prelude::*;

/// The client id the CLI is registered under, matching `oauth.clients` in the
/// API's config. A public client: there is no secret, because a secret shipped
/// inside a binary every user has is not a secret.
const CLIENT_ID: &str = "patr-cli";

/// What the CLI asks for. `offline_access` is the one that matters — without
/// it the token endpoint issues no refresh token, and every command would send
/// the user back to the browser.
const SCOPES: &str = "openid profile email offline_access";

/// The path the browser is redirected back to. The registered URI carries no
/// port, and RFC 8252 section 7.3 has the server ignore the port for loopback
/// redirects, so the CLI is free to bind whatever is available.
const CALLBACK_PATH: &str = "/callback";

/// How long to wait for the browser to come back before giving up.
///
/// Generous, because the user may have to log in and read a consent screen
/// first, but not unbounded: a closed tab should not leave a terminal hanging
/// forever with no way out but Ctrl-C.
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

/// Refresh when the access token has less than this long to live.
///
/// A CLI invocation is seconds long, so a token this fresh at dispatch is
/// still valid when the last request goes out. The margin covers clock skew
/// against the API and a slow command.
const REFRESH_MARGIN: i64 = 60;

/// A token pair as the CLI stores it.
pub struct Tokens {
	/// The access token to present to the API.
	pub access_token: String,
	/// The refresh token that renews it.
	pub refresh_token: Option<String>,
	/// When the access token expires, as unix seconds.
	pub expiry: i64,
}

/// What came back from a refresh attempt.
///
/// The distinction is the point: a rejected refresh token means the grant is
/// gone and the user has to log in again, while a network failure means try
/// later. Collapsing the two would log people out whenever their wifi dropped.
pub enum RefreshOutcome {
	/// A new pair was issued.
	Refreshed(Tokens),
	/// The server said no. The grant is dead — revoked, expired, or replayed.
	Rejected,
}

/// The token endpoint's success body. Only the fields the CLI uses.
#[derive(Debug, Deserialize)]
struct TokenResponse {
	/// The access token.
	access_token: String,
	/// The rotated refresh token. `/token` always sends one back for a grant
	/// with `offline_access`, but the field is optional in the spec.
	refresh_token: Option<String>,
	/// Seconds until `access_token` expires.
	expires_in: i64,
}

/// The OAuth error envelope, per RFC 6749 section 5.2.
#[derive(Debug, Deserialize)]
struct TokenError {
	/// The spec's error code.
	error: String,
	/// A human-readable explanation, when the server sent one.
	error_description: Option<String>,
}

/// Runs the full browser login and returns the tokens it produced.
///
/// The loopback flow of RFC 8252: bind a port on `127.0.0.1`, send the user to
/// `/authorize` with that port as the redirect, and wait for the browser to
/// come back with a code. PKCE binds the code to this process, which is what
/// makes it safe for a public client — a code intercepted on the way back is
/// useless without the verifier, which never leaves this function.
pub async fn login() -> Result<Tokens, AppError> {
	// Bound before the URL is built, because the URL has to name the port.
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.map_err(|err| AppError::OAuthError(format!("could not bind a local port: {err}")))?;
	let port = listener
		.local_addr()
		.map_err(|err| AppError::OAuthError(format!("could not read the local port: {err}")))?
		.port();

	let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");
	let verifier = random_token();
	let challenge = BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
	let csrf_state = random_token();

	let authorize_url = build_authorize_url(&redirect_uri, &challenge, &csrf_state);

	// Printed before the browser is opened, and unconditionally: over SSH or in
	// a container `open` silently does nothing useful, and the URL on screen is
	// the only way through.
	eprintln!("Opening your browser to log in. If it doesn't open, visit:");
	eprintln!();
	eprintln!("  {authorize_url}");
	eprintln!();
	let _ = open::that(&authorize_url);

	let code = tokio::time::timeout(CALLBACK_TIMEOUT, wait_for_callback(&listener, &csrf_state))
		.await
		.map_err(|_| AppError::OAuthError("timed out waiting for the browser".to_owned()))??;

	exchange_code(&code, &verifier, &redirect_uri).await
}

/// Builds the `/authorize` URL the browser is sent to.
fn build_authorize_url(redirect_uri: &str, challenge: &str, csrf_state: &str) -> String {
	let params = serde_qs::to_string(&BTreeMap::from([
		("response_type", "code"),
		("client_id", CLIENT_ID),
		("redirect_uri", redirect_uri),
		("scope", SCOPES),
		("state", csrf_state),
		("code_challenge", challenge),
		// S256 only. OAuth 2.1 forbids `plain`, and the server rejects it.
		("code_challenge_method", "S256"),
	]))
	.expect("failed to encode the authorize query");

	format!("{}/auth/oauth/authorize?{params}", constants::API_BASE_URL)
}

/// What one request to the loopback server turned out to be.
#[derive(Debug, PartialEq, Eq)]
pub enum CallbackOutcome {
	/// Not the callback at all. Browsers open speculative connections and ask
	/// for `/favicon.ico`, so this is normal and the server keeps waiting —
	/// treating the first request that lands as the answer would abandon the
	/// login before it happened.
	NotForUs,
	/// The browser came back with an authorization code.
	Code(String),
	/// The flow ended badly, with the reason to show the user.
	Failed(String),
}

/// Works out what a callback request line means.
///
/// Split from the socket handling because this is the part with the security
/// property in it, and the part worth testing: everything around it is reading
/// a line and writing one back.
pub fn interpret_callback(request_line: &str, csrf_state: &str) -> CallbackOutcome {
	// `GET /callback?code=… HTTP/1.1`
	let target = request_line.split_whitespace().nth(1).unwrap_or_default();

	let Some(query) = target.strip_prefix(CALLBACK_PATH).and_then(|rest| {
		// Exactly the path, optionally with a query. `/callbackery` is not the
		// callback.
		match rest.split_at_checked(1) {
			None => Some(""),
			Some(("?", query)) => Some(query),
			Some(_) => None,
		}
	}) else {
		return CallbackOutcome::NotForUs;
	};

	let params = serde_qs::from_str::<HashMap<String, String>>(query).unwrap_or_default();

	// Checked before the code is looked at. A callback that did not come from
	// the request this process started is somebody else's, and its code must
	// not be exchanged.
	if params.get("state").map(String::as_str) != Some(csrf_state) {
		return CallbackOutcome::Failed(
			"this login didn't come from here — start again with `patr login`".to_owned(),
		);
	}

	if let Some(error) = params.get("error") {
		return CallbackOutcome::Failed(
			params
				.get("error_description")
				.map_or_else(|| error.clone(), Clone::clone),
		);
	}

	params.get("code").map_or_else(
		|| CallbackOutcome::Failed("the callback carried no authorization code".to_owned()),
		|code| CallbackOutcome::Code(code.clone()),
	)
}

/// Serves the loopback callback until the browser arrives with a result.
async fn wait_for_callback(listener: &TcpListener, csrf_state: &str) -> Result<String, AppError> {
	loop {
		let (mut stream, _) = listener
			.accept()
			.await
			.map_err(|err| AppError::OAuthError(format!("callback connection failed: {err}")))?;

		let mut request_line = String::new();
		BufReader::new(&mut stream)
			.read_line(&mut request_line)
			.await
			.map_err(|err| AppError::OAuthError(format!("could not read the callback: {err}")))?;

		match interpret_callback(&request_line, csrf_state) {
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
					"You're logged in. You can close this tab and go back to your terminal.",
				)
				.await;
				return Ok(code);
			}
		}
	}
}

/// Writes one minimal HTML response and closes the connection.
///
/// Best-effort: the user's tokens do not depend on the browser rendering
/// anything, so a write failure here is not worth failing the login over.
async fn respond(stream: &mut tokio::net::TcpStream, status: &str, message: &str) {
	let body = format!(
		"<!doctype html><meta charset=utf-8><title>Patr</title>\
		 <body style=\"font:16px system-ui;margin:4rem auto;max-width:32rem\">{}</body>",
		html_escape(message)
	);
	let response = format!(
		"HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
		 Connection: close\r\n\r\n{body}",
		body.len()
	);

	let _ = stream.write_all(response.as_bytes()).await;
	let _ = stream.flush().await;
}

/// Escapes text for the callback page.
///
/// The message can be an `error_description` chosen by whoever sent the
/// browser here, so it cannot go into the page as-is.
fn html_escape(text: &str) -> String {
	text.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
}

/// Exchanges an authorization code for tokens.
async fn exchange_code(code: &str, verifier: &str, redirect_uri: &str) -> Result<Tokens, AppError> {
	// `redirect_uri` has to be byte-identical to the one sent to `/authorize`
	// — RFC 6749 section 4.1.3 makes the server compare them.
	let form = [
		("grant_type", "authorization_code"),
		("client_id", CLIENT_ID),
		("code", code),
		("redirect_uri", redirect_uri),
		("code_verifier", verifier),
	];

	match post_token(&form).await? {
		Ok(tokens) => Ok(tokens),
		Err(error) => Err(AppError::OAuthError(error.message())),
	}
}

/// Rotates a refresh token.
pub async fn refresh(refresh_token: &str) -> Result<RefreshOutcome, AppError> {
	let form = [
		("grant_type", "refresh_token"),
		("client_id", CLIENT_ID),
		("refresh_token", refresh_token),
	];

	match post_token(&form).await? {
		Ok(tokens) => Ok(RefreshOutcome::Refreshed(tokens)),
		// Only the errors that actually mean the grant is gone clear the
		// session. A rate limit or a 500 is the server having a bad minute,
		// and logging the user out over one would turn a blip into a re-login.
		Err(error)
			if matches!(
				error.error.as_str(),
				"invalid_grant" | "invalid_client" | "unauthorized_client"
			) =>
		{
			Ok(RefreshOutcome::Rejected)
		}
		Err(error) => Err(AppError::OAuthError(error.message())),
	}
}

/// Ends the grant behind a refresh token.
///
/// Best-effort, and deliberately so: `patr logout` must clear the local state
/// whether or not the API is reachable, or a user on a plane could never log
/// out. RFC 7009 makes an unknown token a success, so a token already revoked
/// server-side is not an error either.
pub async fn revoke(refresh_token: &str) {
	let form = [
		("token", refresh_token),
		("token_type_hint", "refresh_token"),
		("client_id", CLIENT_ID),
	];

	let result = post_form(
		&format!("{}/auth/oauth/revoke", constants::API_BASE_URL),
		&form,
	)
	.await;

	match result {
		Ok(response) if response.status().is_success() => {}
		Ok(response) => {
			debug!("The API refused to revoke the grant: {}", response.status());
		}
		Err(err) => {
			debug!("Could not reach the API to revoke the grant: {}", err);
		}
	}
}

/// POSTs an `application/x-www-form-urlencoded` body.
///
/// Hand-encoded rather than through `RequestBuilder::form`, which this build of
/// reqwest does not have — the workspace turns its default features off, and
/// the form helper is behind one of them.
async fn post_form(url: &str, form: &[(&str, &str)]) -> Result<reqwest::Response, AppError> {
	reqwest::Client::new()
		.post(url)
		.header(reqwest::header::USER_AGENT, constants::USER_AGENT.as_str())
		.header(
			reqwest::header::CONTENT_TYPE,
			"application/x-www-form-urlencoded",
		)
		.body(
			serde_qs::to_string(&form.iter().copied().collect::<BTreeMap<_, _>>())
				.expect("failed to encode the form body"),
		)
		.send()
		.await
		.map_err(Into::into)
}

/// POSTs a form to `/token` and sorts the two kinds of failure apart.
///
/// The outer `Result` is transport: the request never got an answer. The inner
/// one is the protocol: the server answered, and said no.
async fn post_token(form: &[(&str, &str)]) -> Result<Result<Tokens, TokenError>, AppError> {
	let response = post_form(
		&format!("{}/auth/oauth/token", constants::API_BASE_URL),
		form,
	)
	.await?;

	let status = response.status();
	let body = response.text().await?;

	if !status.is_success() {
		return Ok(Err(serde_json::from_str::<TokenError>(&body).unwrap_or(
			TokenError {
				error: format!("the token endpoint returned {status}"),
				error_description: None,
			},
		)));
	}

	let token = serde_json::from_str::<TokenResponse>(&body)
		.map_err(|err| AppError::OAuthError(format!("could not read the token response: {err}")))?;

	Ok(Ok(Tokens {
		access_token: token.access_token,
		refresh_token: token.refresh_token,
		expiry: OffsetDateTime::now_utc().unix_timestamp() + token.expires_in,
	}))
}

impl TokenError {
	/// The message to show the user.
	fn message(&self) -> String {
		self.error_description
			.clone()
			.unwrap_or_else(|| self.error.clone())
	}
}

/// Whether an access token expiring at `expiry` should be renewed now.
///
/// Split out so the decision is testable without a clock or a server.
pub const fn needs_refresh(expiry: Option<i64>, now: i64) -> bool {
	match expiry {
		// No recorded expiry means a token from before this field existed.
		// Refreshing is the safe read: the worst case is one extra round trip.
		None => true,
		Some(expiry) => expiry - now < REFRESH_MARGIN,
	}
}

/// 32 random bytes, base64url-encoded.
///
/// Used for both the PKCE verifier and the CSRF `state`. 43 characters, which
/// is inside the 43–128 RFC 7636 section 4.1 allows for a verifier.
fn random_token() -> String {
	BASE64_URL_SAFE_NO_PAD.encode(rand::rng().random::<[u8; 32]>())
}
