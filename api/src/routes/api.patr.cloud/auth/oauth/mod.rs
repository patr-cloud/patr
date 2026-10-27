use std::collections::BTreeMap;

use axum::Router;

use crate::prelude::*;

/// The authorization endpoint.
mod authorize;
/// The OAuth errors that go back to a client.
mod error;
/// Reads the pending authorization request behind a consent screen.
mod get_consent_request;
/// Records the user's decision and hands back where to send them.
mod submit_consent;

use self::{authorize::*, get_consent_request::*, submit_consent::*};

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
	Router::new()
		.mount_endpoint(authorize, state, host_client_types)
		.mount_auth_endpoint(get_consent_request, state, host_client_types)
		.mount_auth_endpoint(submit_consent, state, host_client_types)
}
