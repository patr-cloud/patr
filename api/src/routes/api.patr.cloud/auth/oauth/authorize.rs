use std::{collections::HashMap, net::IpAddr};

use axum::{
	extract::{RawQuery, State},
	response::{IntoResponse, Redirect, Response},
};
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use super::error::{AuthorizeError, OAuthErrorCode};
use crate::{
	models::oauth::types::{CodeChallengeMethod, OAuthAuthorizationRequest, SUPPORTED_SCOPES},
	prelude::*,
	utils::{config::OAuthClientConfig, extractors::ClientIP},
};

/// The authorization endpoint: `GET /auth/oauth/authorize`.
///
/// This is the front channel — a browser lands here, having been sent by the
/// client. It validates everything it can without a session, parks the
/// request in Redis, and bounces the browser to the dashboard's consent
/// screen carrying nothing but an opaque request id.
///
/// It cannot read the user's session: the cookie lives on the dashboard's
/// origin and this endpoint is served from the API's. Binding the user is
/// therefore the consent endpoint's job, not this one's.
#[instrument(skip(state))]
pub async fn authorize(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	RawQuery(query): RawQuery,
) -> Response {
	// Before the query is even parsed, and reported as a plain response
	// rather than a redirect: everything below can bounce an error back to
	// the client's `redirect_uri`, and a redirect that fires on a throttled
	// request would make this endpoint an amplifier.
	if let Err(error) = super::enforce_rate_limit(&state, client_ip).await {
		return error.into_response();
	}

	match handle(&state, query.as_deref().unwrap_or_default()).await {
		Ok(response) => response,
		Err(error) => error.into_response(),
	}
}

/// Validates the request and parks it, or explains why it could not.
async fn handle(state: &AppState, raw_query: &str) -> Result<Response, AuthorizeError> {
	// Parsed permissively first. A strict parse that rejected the whole query
	// would leave us unable to read `redirect_uri` and `state` — and without
	// those, a malformed request could only be reported as a bare 400, where
	// the spec wants it redirected back to the client as `invalid_request`.
	let params = serde_qs::from_str::<HashMap<String, String>>(raw_query).map_err(|err| {
		AuthorizeError::untrusted(
			OAuthErrorCode::InvalidRequest,
			format!("could not parse the query string: {err}"),
		)
	})?;

	let client_id = params.get("client_id").ok_or_else(|| {
		AuthorizeError::untrusted(OAuthErrorCode::InvalidRequest, "`client_id` is required")
	})?;

	let client = state.config.oauth.clients.get(client_id).ok_or_else(|| {
		// Deliberately vague: an attacker enumerating client ids learns
		// nothing from the difference between "unknown" and "disabled".
		AuthorizeError::untrusted(OAuthErrorCode::InvalidClient, "unknown `client_id`")
	})?;

	// Everything above this point is reported without redirecting, because
	// the redirect target has not been established yet. Everything below can
	// safely bounce back to the client.
	let redirect_uri =
		resolve_redirect_uri(client, params.get("redirect_uri").map(String::as_str))?;

	let oauth_state = params.get("state").cloned();

	let response_type = params.get("response_type").map(String::as_str);
	if response_type != Some("code") {
		return Err(AuthorizeError::redirect(
			&redirect_uri,
			oauth_state,
			OAuthErrorCode::UnsupportedResponseType,
			"`response_type` must be `code`",
		));
	}

	let scopes =
		resolve_scopes(client, params.get("scope").map(String::as_str)).map_err(|message| {
			AuthorizeError::redirect(
				&redirect_uri,
				oauth_state.clone(),
				OAuthErrorCode::InvalidScope,
				message,
			)
		})?;

	// PKCE is mandatory for every client under OAuth 2.1, confidential ones
	// included — not just the public clients RFC 7636 originally targeted.
	let code_challenge = params
		.get("code_challenge")
		.filter(|challenge| !challenge.is_empty())
		.cloned()
		.ok_or_else(|| {
			AuthorizeError::redirect(
				&redirect_uri,
				oauth_state.clone(),
				OAuthErrorCode::InvalidRequest,
				"`code_challenge` is required",
			)
		})?;

	if params.get("code_challenge_method").map(String::as_str) != Some("S256") {
		return Err(AuthorizeError::redirect(
			&redirect_uri,
			oauth_state,
			OAuthErrorCode::InvalidRequest,
			"`code_challenge_method` must be `S256`",
		));
	}

	let request = OAuthAuthorizationRequest {
		client_id: client_id.clone(),
		redirect_uri,
		scopes,
		state: oauth_state.clone(),
		nonce: params.get("nonce").cloned(),
		code_challenge,
		code_challenge_method: CodeChallengeMethod::S256,
		created: OffsetDateTime::now_utc(),
	};

	let request_id = Uuid::now_v1();
	let redis = state.redis.clone();

	let payload = serde_json::to_string(&request).map_err(|err| {
		error!("Error serialising an authorization request: {}", err);
		AuthorizeError::redirect(
			&request.redirect_uri,
			oauth_state.clone(),
			OAuthErrorCode::ServerError,
			"could not store the authorization request",
		)
	})?;

	redis
		.setex(
			redis::keys::oauth_authorization_request(&request_id),
			constants::OAUTH_AUTHORIZATION_REQUEST_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			payload,
		)
		.await
		.map_err(|err| {
			error!("Error parking an authorization request in Redis: {}", err);
			AuthorizeError::redirect(
				&request.redirect_uri,
				oauth_state,
				OAuthErrorCode::ServerError,
				"could not store the authorization request",
			)
		})?;

	Ok(Redirect::to(&format!(
		"{}/authorize?requestId={}",
		super::dashboard_url(&state.config),
		request_id
	))
	.into_response())
}

/// Picks the redirect URI to use, and checks the client is allowed to use it.
///
/// Matching is exact, with one exception: a loopback URI's port is ignored.
/// A native client binds an ephemeral port it cannot know when it registers,
/// which is what RFC 8252 section 7.3 carves out. Everything else — scheme,
/// host, path, query — still has to match exactly, so the carve-out cannot be
/// used to redirect somewhere else on the same host.
fn resolve_redirect_uri(
	client: &OAuthClientConfig,
	requested: Option<&str>,
) -> Result<String, AuthorizeError> {
	let Some(requested) = requested else {
		// Omitting it is only unambiguous when the client registered exactly
		// one, which is the single case the spec allows it in.
		return match client.redirect_uris.as_slice() {
			[only] => Ok(only.clone()),
			_ => Err(AuthorizeError::untrusted(
				OAuthErrorCode::InvalidRequest,
				"`redirect_uri` is required when the client registers more than one",
			)),
		};
	};

	let matches = client
		.redirect_uris
		.iter()
		.any(|registered| redirect_uris_match(registered, requested));

	if matches {
		Ok(requested.to_owned())
	} else {
		Err(AuthorizeError::untrusted(
			OAuthErrorCode::InvalidRequest,
			"`redirect_uri` does not match one registered for this client",
		))
	}
}

/// Whether a requested redirect URI matches a registered one.
fn redirect_uris_match(registered: &str, requested: &str) -> bool {
	if registered == requested {
		return true;
	}

	let (Ok(registered), Ok(requested)) = (
		reqwest::Url::parse(registered),
		reqwest::Url::parse(requested),
	) else {
		return false;
	};

	// Only loopback gets the relaxed treatment, and only on the port.
	if !is_loopback(&registered) || !is_loopback(&requested) {
		return false;
	}

	registered.scheme() == requested.scheme() &&
		registered.host() == requested.host() &&
		registered.path() == requested.path() &&
		registered.query() == requested.query()
}

/// Whether a URL points at the local machine by literal address.
///
/// Deliberately does not accept `localhost`: RFC 8252 section 8.3 warns that
/// it resolves through the host's name lookup, which is not always under the
/// user's control, so only literal loopback addresses get the relaxed port
/// treatment.
fn is_loopback(url: &reqwest::Url) -> bool {
	url.host_str()
		.and_then(|host| {
			host.trim_start_matches('[')
				.trim_end_matches(']')
				.parse::<IpAddr>()
				.ok()
		})
		.is_some_and(|addr| addr.is_loopback())
}

/// Resolves the requested scopes, defaulting to `openid` when none are asked
/// for, and rejecting anything this client may not request.
fn resolve_scopes(
	client: &OAuthClientConfig,
	requested: Option<&str>,
) -> Result<Vec<String>, String> {
	let requested = requested
		.map(str::trim)
		.filter(|scope| !scope.is_empty())
		.unwrap_or("openid");

	let mut scopes = Vec::new();
	for scope in requested.split_whitespace() {
		if !SUPPORTED_SCOPES.contains(&scope) {
			return Err(format!("`{scope}` is not a scope this server issues"));
		}
		if !client.allowed_scopes.iter().any(|allowed| allowed == scope) {
			return Err(format!("this client may not request `{scope}`"));
		}
		if !scopes.iter().any(|existing| existing == scope) {
			scopes.push(scope.to_owned());
		}
	}

	// Every flow here is an OIDC one; without `openid` there would be no id
	// token and nothing for the consent screen to describe.
	if !scopes.iter().any(|scope| scope == "openid") {
		return Err("`openid` must be among the requested scopes".to_owned());
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
		let registered = "http://127.0.0.1/callback";

		assert!(redirect_uris_match(
			registered,
			"http://127.0.0.1:49152/callback"
		));
		assert!(redirect_uris_match(
			registered,
			"http://127.0.0.1:1/callback"
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
				!redirect_uris_match(registered, requested),
				"`{requested}` should not match `{registered}`"
			);
		}
	}

	/// The IPv6 loopback gets the same treatment, written the same way.
	#[test]
	fn ipv6_loopback_ignores_the_port() {
		let registered = "http://[::1]/callback";

		assert!(redirect_uris_match(
			registered,
			"http://[::1]:49152/callback"
		));
		assert!(!redirect_uris_match(
			registered,
			"http://[::2]:49152/callback"
		));
	}
}
