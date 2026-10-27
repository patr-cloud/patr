use std::str::FromStr;

use axum::http::StatusCode;
use models::{api::auth::oauth::*, utils::Location};
use rustis::commands::{SetExpiration, StringCommands};
use url::{Host, Url};

use super::error::{OAuthError, OAuthErrorCode};
use crate::{
	models::oauth::types::{OAuthAuthorizationRequest, SUPPORTED_SCOPES},
	prelude::*,
};

/// An unknown client or unregistered redirect URI is a plain error, never a
/// redirect (RFC 6749 section 4.1.2.1). Every other failure goes back to the
/// client.
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

	let mut scopes = Vec::<String>::new();
	for scope in scope.as_deref().unwrap_or_default().split_whitespace() {
		if !SUPPORTED_SCOPES.contains(&scope) {
			return deny(OAuthError::new(
				OAuthErrorCode::InvalidScope,
				format!("`{scope}` is not a scope this server issues"),
			));
		}
		if !client.allowed_scopes.iter().any(|allowed| allowed == scope) {
			return deny(OAuthError::new(
				OAuthErrorCode::InvalidScope,
				format!("this client may not request `{scope}`"),
			));
		}
		if !scopes.iter().any(|existing| existing == scope) {
			scopes.push(scope.to_owned());
		}
	}

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
		.set_with_options(
			redis::keys::oauth_authorization_request(&request_id),
			serde_json::to_string(&OAuthAuthorizationRequest {
				client_id,
				redirect_uri,
				scopes,
				state: oauth_state,
				nonce,
				code_challenge,
			})?,
			None,
			SetExpiration::Ex(
				constants::OAUTH_AUTHORIZATION_REQUEST_VALIDITY
					.whole_seconds()
					.unsigned_abs(),
			),
		)
		.await?;

	// Cloud serves the dashboard on `app.`; self-hosted off the base domain.
	let base_domain = &state.config.server.base_domain;
	let dashboard_url = if cfg!(feature = "cloud") {
		format!("https://app.{base_domain}")
	} else {
		format!("https://{base_domain}")
	};
	redirect_to(format!("{dashboard_url}/authorize?requestId={request_id}"))
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

/// Whether a requested redirect URI matches a registered one: exactly, except
/// that a literal loopback address ignores the port (RFC 8252 sections 7.3
/// and 8.3).
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
