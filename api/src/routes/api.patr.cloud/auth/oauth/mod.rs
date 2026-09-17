use std::collections::BTreeMap;

use axum::Router;

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

use self::{authorize::*, get_consent_request::*, submit_consent::*};

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
		.mount_endpoint(authorize, state, allowed_client_type)
		.mount_auth_endpoint(get_consent_request, state, allowed_client_type)
		.mount_auth_endpoint(submit_consent, state, allowed_client_type)
}
