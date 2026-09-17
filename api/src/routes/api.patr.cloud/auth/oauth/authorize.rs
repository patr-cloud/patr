use std::str::FromStr;

use axum::http::StatusCode;
use models::{api::auth::oauth::*, utils::Location};
use rustis::commands::StringCommands;
use url::{Host, Url};

use super::error::{OAuthError, OAuthErrorCode};
use crate::{
	models::oauth::types::{OAuthAuthorizationRequest, SUPPORTED_SCOPES},
	prelude::*,
	utils::config::OAuthClientConfig,
};

/// The authorization endpoint.
///
/// An unknown client, or a redirect URI it never registered, is a plain
/// error: RFC 6749 section 4.1.2.1 forbids bouncing the browser to an
/// unverified URI. Once the URI is known to be the client's, every other
/// failure goes back to it as an OAuth error, since that is the only way the
/// client learns what happened.
pub async fn authorize(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: OAuthAuthorizePath,
				query:
					OAuthAuthorizeQueryProcessed {
						client_id,
						redirect_uri,
						response_type,
						scope,
						state: oauth_state,
						nonce,
						code_challenge,
						code_challenge_method,
					},
				headers: (),
				body: OAuthAuthorizeRequestProcessed,
			},
		database: _,
		redis,
		client_ip: _,
		state,
	}: AppRequest<'_, OAuthAuthorizeRequest>,
) -> Result<AppResponse<OAuthAuthorizeRequest>, ErrorType> {
	trace!("Authorization request from client `{}`", client_id);

	let Some(client) = state.config.oauth.clients.get(&client_id) else {
		debug!("Unknown OAuth client `{}`", client_id);
		return Err(ErrorType::WrongParameters);
	};

	// OAuth 2.1 section 3.1.2.3: sent, it must match a registered URI;
	// omitted, the client must have registered exactly one.
	let redirect_uri = match (redirect_uri, client.redirect_uris.as_slice()) {
		(Some(requested), registered)
			if registered
				.iter()
				.any(|registered| redirect_uris_match(registered, &requested)) =>
		{
			requested
		}
		(None, [only]) => only.clone(),
		_ => {
			debug!("Rejected redirect URI for client `{}`", client_id);
			return Err(ErrorType::WrongParameters);
		}
	};

	let deny = |error: OAuthError| {
		let mut params = error.as_query_params();
		// Echoed verbatim, and omitted entirely when the client sent none —
		// `state=` with an empty value is not the same thing.
		if let Some(oauth_state) = &oauth_state {
			params.push(("state", oauth_state.clone()));
		}
		redirect_to(super::append_query_params(&redirect_uri, &params))
	};

	if response_type.as_deref() != Some("code") {
		return deny(OAuthError::new(
			OAuthErrorCode::UnsupportedResponseType,
			"`response_type` must be `code`",
		));
	}

	let scopes = match resolve_scopes(client, scope.as_deref()) {
		Ok(scopes) => scopes,
		Err(error) => return deny(error),
	};

	// PKCE is mandatory for every client under OAuth 2.1, confidential ones
	// included.
	let Some(code_challenge) = code_challenge.filter(|challenge| !challenge.is_empty()) else {
		return deny(OAuthError::new(
			OAuthErrorCode::InvalidRequest,
			"`code_challenge` is required",
		));
	};
	if code_challenge_method.as_deref() != Some("S256") {
		return deny(OAuthError::new(
			OAuthErrorCode::InvalidRequest,
			"`code_challenge_method` must be `S256`",
		));
	}

	let request_id = Uuid::new_v4();

	redis
		.setex(
			redis::keys::oauth_authorization_request(&request_id),
			constants::OAUTH_AUTHORIZATION_REQUEST_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			serde_json::to_string(&OAuthAuthorizationRequest {
				client_id,
				redirect_uri,
				scopes,
				state: oauth_state,
				nonce,
				code_challenge,
			})?,
		)
		.await?;

	redirect_to(format!(
		"{}/authorize?requestId={}",
		super::dashboard_url(&state.config),
		request_id
	))
}

/// Sends the browser to `location`.
fn redirect_to(location: String) -> Result<AppResponse<OAuthAuthorizeRequest>, ErrorType> {
	AppResponse::builder()
		.body(OAuthAuthorizeResponse)
		.headers(OAuthAuthorizeResponseHeaders {
			location: Location::from_str(&location)?,
		})
		.status_code(StatusCode::SEE_OTHER)
		.build()
		.into_result()
}

/// Whether a requested redirect URI matches a registered one.
///
/// Exact, except that a loopback URI's port is ignored: a native client binds
/// an ephemeral one it cannot know when it registers (RFC 8252 section 7.3).
/// Only literal addresses count — `localhost` resolves through name lookup,
/// which is not always the user's to control (section 8.3).
fn redirect_uris_match(registered: &str, requested: &str) -> bool {
	if registered == requested {
		return true;
	}

	let (Ok(mut registered), Ok(mut requested)) = (Url::parse(registered), Url::parse(requested))
	else {
		return false;
	};

	let loopback = match registered.host() {
		Some(Host::Ipv4(ip)) => ip.is_loopback(),
		Some(Host::Ipv6(ip)) => ip.is_loopback(),
		_ => false,
	};

	// Equality covers the host too, so a non-loopback request cannot match.
	loopback &&
		registered.set_port(None).is_ok() &&
		requested.set_port(None).is_ok() &&
		registered == requested
}

/// Resolves the requested scopes, rejecting anything this client may not ask
/// for. None at all is a plain OAuth request: no id token, and nothing but the
/// access token to consent to.
fn resolve_scopes(
	client: &OAuthClientConfig,
	requested: Option<&str>,
) -> Result<Vec<String>, OAuthError> {
	let mut scopes = Vec::<String>::new();
	for scope in requested.unwrap_or_default().split_whitespace() {
		if !SUPPORTED_SCOPES.contains(&scope) {
			return Err(OAuthError::new(
				OAuthErrorCode::InvalidScope,
				format!("`{scope}` is not a scope this server issues"),
			));
		}
		if !client.allowed_scopes.iter().any(|allowed| allowed == scope) {
			return Err(OAuthError::new(
				OAuthErrorCode::InvalidScope,
				format!("this client may not request `{scope}`"),
			));
		}
		if !scopes.iter().any(|existing| existing == scope) {
			scopes.push(scope.to_owned());
		}
	}

	Ok(scopes)
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Exact matching is the rule, and the one that stops an attacker
	/// nominating their own redirect target.
	#[test]
	fn non_loopback_uris_must_match_exactly() {
		let registered = "https://grafana.patr.cloud/login/generic_oauth";

		assert!(redirect_uris_match(registered, registered));

		for requested in [
			"https://grafana.patr.cloud/login/generic_oauth/",
			"https://grafana.patr.cloud/login/other",
			"https://evil.example/login/generic_oauth",
			"http://grafana.patr.cloud/login/generic_oauth",
			"https://grafana.patr.cloud:8443/login/generic_oauth",
			"https://grafana.patr.cloud/login/generic_oauth?next=x",
		] {
			assert!(
				!redirect_uris_match(registered, requested),
				"`{requested}` should not match `{registered}`"
			);
		}
	}

	/// RFC 8252 section 7.3: a native client binds an ephemeral port it
	/// cannot know when it registers, so the port — and only the port — is
	/// ignored for loopback addresses.
	#[test]
	fn loopback_uris_ignore_the_port_and_nothing_else() {
		assert!(redirect_uris_match(
			"http://127.0.0.1/callback",
			"http://127.0.0.1:49152/callback"
		));
		assert!(redirect_uris_match(
			"http://[::1]/callback",
			"http://[::1]:49152/callback"
		));

		for requested in [
			// A different path on the same host is still a different target.
			"http://127.0.0.1:49152/other",
			// `localhost` resolves through name lookup, which RFC 8252
			// section 8.3 warns is not always the user's to control.
			"http://localhost:49152/callback",
			// A different loopback family is registered separately.
			"http://[::1]:49152/callback",
			// The relaxation must not reach beyond the local machine.
			"http://10.0.0.1:49152/callback",
			"http://evil.example:49152/callback",
		] {
			assert!(
				!redirect_uris_match("http://127.0.0.1/callback", requested),
				"`{requested}` should not match `http://127.0.0.1/callback`"
			);
		}
	}
}
