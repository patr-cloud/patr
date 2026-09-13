use std::{any::Any, collections::BTreeMap, net::IpAddr};

use axum::{
	Router,
	response::{IntoResponse, Response},
	routing::post,
};
use tower_http::catch_panic::CatchPanicLayer;

use self::error::{OAuthError, OAuthErrorCode};
use crate::{prelude::*, utils::config::AppConfig};

/// The authorization endpoint. The front channel: a browser arrives here
/// from the client, and leaves for the consent screen.
mod authorize;
/// The spec's error shape, for what goes back to a client.
mod error;
/// Reads the pending authorization request behind a consent screen.
mod get_consent_request;
/// Records the user's decision and hands back where to send them.
mod submit_consent;
/// The token endpoint. The back channel: a client's own server exchanges a
/// code, or rotates a refresh token, here.
mod token;

use self::{authorize::*, get_consent_request::*, submit_consent::*};

/// Applies the API's rate limits to a raw OAuth route.
///
/// The back-channel endpoints are plain axum routes, so they never pass through
/// `RateLimiterLayer` the way a `declare_api_endpoint!` one does — which left
/// the whole OAuth surface unthrottled. `/token` is the reason this matters:
/// it takes a client secret and a refresh token secret, and is reachable
/// without a session.
///
/// Keyed per-IP, with the same windows as the rest of the API rather than a
/// second set to keep in step. Note that a server-to-server client puts all of
/// its users' traffic on one IP, so tightening these would throttle a busy
/// first-party client long before it inconvenienced an attacker.
pub async fn enforce_rate_limit(state: &AppState, client_ip: IpAddr) -> Result<(), OAuthError> {
	crate::models::rate_limiter::check_rate_limit(
		&mut state.redis.clone(),
		client_ip,
		None,
		&crate::utils::layers::RATE_LIMITS,
	)
	.await
	.map_err(|err| {
		warn!("Rate limit tripped on an OAuth endpoint: {}", err);
		OAuthError::new(
			OAuthErrorCode::TemporarilyUnavailable,
			"too many requests; slow down and try again",
		)
	})
}

/// Renders a panic on a protocol endpoint in the spec's error shape, since a
/// client library would not understand Patr's.
fn on_protocol_panic(panic: Box<dyn Any + Send>) -> Response {
	let details = panic
		.downcast_ref::<&str>()
		.map(|message| (*message).to_owned())
		.or_else(|| panic.downcast_ref::<String>().cloned())
		.unwrap_or_else(|| "unknown panic".to_owned());
	OAuthError::server_error(format!("caught panic while handling request: {details}"))
		.into_response()
}

/// The dashboard's origin, where the browser is sent for consent.
///
/// Cloud serves it on `app.`; self-hosted off the base domain directly.
pub fn dashboard_url(config: &AppConfig) -> String {
	let base_domain = &config.server.base_domain;
	if cfg!(feature = "cloud") {
		format!("https://app.{base_domain}")
	} else {
		format!("https://{base_domain}")
	}
}

/// The OIDC issuer identifier.
///
/// Everything derives from this: the `iss` claim in every token, and the URL
/// a client fetches the discovery document from. Cloud serves the API on its
/// own subdomain; self-hosted path-routes it under `/api`.
pub fn issuer(config: &AppConfig) -> String {
	let base_domain = &config.server.base_domain;
	if cfg!(feature = "cloud") {
		format!("https://api.{base_domain}")
	} else {
		format!("https://{base_domain}/api")
	}
}

/// The audience an access token must name to be accepted by the API.
///
/// The same value as the issuer, because the API is both. What matters is
/// that it differs from an id token's audience — which is the client — so
/// the two cannot be swapped.
pub fn api_audience(config: &AppConfig) -> String {
	issuer(config)
}

/// Appends query parameters to a URI, keeping whatever it already carries.
///
/// A registered `redirect_uri` is allowed its own query string, and RFC 6749
/// section 3.1.2 says the response parameters are added to it rather than
/// replacing it — so this cannot just format a `?` on.
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
pub async fn setup_routes(state: &AppState, allowed_client_type: ClientType) -> Router {
	Router::new()
		.route("/auth/oauth/token", post(token::token))
		.layer(CatchPanicLayer::custom(on_protocol_panic))
		.with_state(state.clone())
		.mount_endpoint(authorize, state, allowed_client_type)
		.mount_auth_endpoint(get_consent_request, state, allowed_client_type)
		.mount_auth_endpoint(submit_consent, state, allowed_client_type)
}
