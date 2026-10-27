use std::{any::Any, collections::BTreeMap, net::IpAddr};

use axum::{
	Router,
	response::{IntoResponse, Response},
	routing::post,
};
use tower_http::catch_panic::CatchPanicLayer;

use self::error::{OAuthError, OAuthErrorCode};
use crate::prelude::*;

/// The authorization endpoint.
mod authorize;
/// The OAuth errors that go back to a client.
mod error;
/// Reads the pending authorization request behind a consent screen.
mod get_consent_request;
/// Records the user's decision and hands back where to send them.
mod submit_consent;
/// The token endpoint.
mod token;

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
				.route("/auth/oauth/token", post(token::token))
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
