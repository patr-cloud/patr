use std::{
	any::Any,
	collections::{BTreeMap, HashMap},
	net::IpAddr,
};

use axum::{
	Router,
	http::HeaderMap,
	response::{IntoResponse, Response},
	routing::{get, post},
};
use headers::{Authorization, HeaderMapExt, authorization::Basic};
use sha2::{Digest, Sha256};
use tower_http::catch_panic::CatchPanicLayer;

use self::error::{OAuthError, OAuthErrorCode};
use crate::prelude::*;

/// The authorization endpoint.
mod authorize;
/// The provider metadata and the JWKS.
mod discovery;
/// The OAuth errors that go back to a client.
mod error;
/// Reads the pending authorization request behind a consent screen.
mod get_consent_request;
/// The introspection endpoint.
mod introspect;
/// The revocation endpoint.
mod revoke;
/// Records the user's decision and hands back where to send them.
mod submit_consent;
/// The token endpoint.
mod token;
/// The UserInfo endpoint.
mod userinfo;

use self::{authorize::*, get_consent_request::*, submit_consent::*};

/// Applies the API's per-IP rate limits to a raw OAuth route, which the
/// endpoint layer stack never sees.
pub async fn enforce_rate_limit(state: &AppState, client_ip: IpAddr) -> Result<(), OAuthError> {
	crate::models::rate_limiter::check_rate_limit(
		&mut state.redis.clone(),
		client_ip,
		None,
		&crate::utils::layers::RATE_LIMITS,
	)
	.await
	.map_err(|_| {
		OAuthError::new(
			OAuthErrorCode::TemporarilyUnavailable,
			"too many requests; slow down and try again",
		)
	})
}

/// Authenticates the client and returns its id. Takes credentials over Basic
/// and in the form (RFC 6749 section 2.3.1), since Grafana's client tries
/// Basic first. A public client presents none, and relies on PKCE.
pub fn authenticate_client(
	state: &AppState,
	headers: &HeaderMap,
	client_id: Option<&str>,
	client_secret: Option<&str>,
) -> Result<String, OAuthError> {
	// Both halves of Basic are form-encoded before being base64'd.
	let decode = |value: &str| {
		serde_qs::from_str::<HashMap<String, String>>(&format!("v={value}"))
			.ok()
			.and_then(|mut map| map.remove("v"))
			.unwrap_or_else(|| value.to_owned())
	};

	let (client_id, presented_secret) =
		match (headers.typed_get::<Authorization<Basic>>(), client_id) {
			(Some(basic), _) => (decode(basic.username()), Some(decode(basic.password()))),
			(None, Some(client_id)) => (client_id.to_owned(), client_secret.map(str::to_owned)),
			(None, None) => {
				return Err(OAuthError::new(
					OAuthErrorCode::InvalidClient,
					"no client credentials were presented",
				));
			}
		};

	let Some(client) = state.config.oauth.clients.get(&client_id) else {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidClient,
			"unknown client",
		));
	};

	match (&client.client_secret, presented_secret) {
		// Compared by digest, so the time taken doesn't leak how much matched.
		(Some(expected), Some(presented))
			if Sha256::digest(expected) == Sha256::digest(&presented) => {}
		(None, None) => {}
		_ => {
			return Err(OAuthError::new(
				OAuthErrorCode::InvalidClient,
				"client authentication failed",
			));
		}
	}

	Ok(client_id)
}

/// Renders a panic on a raw OAuth route as a `server_error`, since a client
/// library wouldn't understand Patr's error.
fn on_protocol_panic(panic: Box<dyn Any + Send>) -> Response {
	let details = panic
		.downcast_ref::<&str>()
		.map(|message| (*message).to_owned())
		.or_else(|| panic.downcast_ref::<String>().cloned())
		.unwrap_or_else(|| "unknown panic".to_owned());
	OAuthError::server_error(format!("caught panic while handling request: {details}"))
		.into_response()
}

/// Appends query parameters to a URI, keeping any query it already has (RFC
/// 6749 section 3.1.2).
pub fn append_query_params(uri: &str, params: &[(&str, String)]) -> String {
	let encoded = serde_qs::to_string(&params.iter().cloned().collect::<BTreeMap<_, _>>())
		.unwrap_or_default();
	if encoded.is_empty() {
		return uri.to_owned();
	}

	let separator = if uri.contains('?') { '&' } else { '?' };
	format!("{uri}{separator}{encoded}")
}

/// Sets up the OAuth routes.
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState, host_client_types: &[ActorClientType]) -> Router {
	// Raw routes skip `mount_*`'s filtering, so they're only mounted where an
	// OAuth client can call them.
	let protocol_routes =
		if host_client_types.contains(&ActorClientType::UserLogin(UserLoginType::OAuthLogin)) {
			Router::new()
				// Relative to the API root, so both hang off the issuer.
				.route(
					"/.well-known/openid-configuration",
					get(discovery::openid_configuration),
				)
				.route("/.well-known/jwks.json", get(discovery::jwks))
				.route("/auth/oauth/token", post(token::token))
				.route("/auth/oauth/revoke", post(revoke::revoke))
				.route("/auth/oauth/introspect", post(introspect::introspect))
				// OIDC Core section 5.3 requires both methods.
				.route(
					"/auth/oauth/userinfo",
					get(userinfo::userinfo).post(userinfo::userinfo),
				)
				.layer(CatchPanicLayer::custom(on_protocol_panic))
				.with_state(state.clone())
		} else {
			Router::new()
		};

	protocol_routes
		.mount_endpoint(authorize, state, host_client_types)
		.mount_auth_endpoint(get_consent_request, state, host_client_types)
		.mount_auth_endpoint(submit_consent, state, host_client_types)
}
