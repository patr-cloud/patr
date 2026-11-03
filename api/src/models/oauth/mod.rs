/// The claims carried by the tokens this provider issues.
pub mod claims;
/// The keys that sign those tokens.
pub mod keys;
/// The payloads the authorization code flow parks in Redis.
pub mod types;

use crate::utils::config::AppConfig;

/// The OIDC issuer, and the audience of the access tokens the API accepts.
/// Cloud serves the API on its own subdomain; self-hosted under `/api`.
pub fn issuer(config: &AppConfig) -> String {
	let base_domain = &config.server.base_domain;
	if cfg!(feature = "cloud") {
		format!("https://api.{base_domain}")
	} else {
		format!("https://{base_domain}/api")
	}
}
