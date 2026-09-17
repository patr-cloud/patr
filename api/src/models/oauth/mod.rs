/// The claims carried by the tokens this provider issues.
pub mod claims;
/// The identity claims a grant's scopes entitle a client to see, shared by
/// `/userinfo` and the id token.
pub mod identity;
/// Signing keys and the JWKS.
pub mod keys;
/// The payloads parked in Redis between the front and back channels of the
/// authorization code flow.
pub mod types;

use crate::{prelude::*, utils::config::AppConfig};

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
