use axum::{Json, extract::State};

use super::error::OAuthError;
use crate::{
	models::oauth::{self, keys, types::SUPPORTED_SCOPES},
	prelude::*,
	utils::extractors::ClientIP,
};

/// `GET /.well-known/openid-configuration`: the provider metadata (OIDC
/// Discovery section 3). Lists only what's mounted, since a client library
/// will call any endpoint it's given.
pub async fn openid_configuration(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
) -> Result<Json<serde_json::Value>, OAuthError> {
	super::enforce_rate_limit(&state, client_ip).await?;

	let issuer = oauth::issuer(&state.config);

	Ok(Json(serde_json::json!({
		"issuer": issuer,
		"authorization_endpoint": format!("{issuer}/auth/oauth/authorize"),
		"token_endpoint": format!("{issuer}/auth/oauth/token"),
		"userinfo_endpoint": format!("{issuer}/auth/oauth/userinfo"),
		"jwks_uri": format!("{issuer}/.well-known/jwks.json"),
		"response_types_supported": ["code"],
		"subject_types_supported": ["public"],
		"id_token_signing_alg_values_supported": ["ES256"],
		"scopes_supported": SUPPORTED_SCOPES,
		"grant_types_supported": ["authorization_code", "refresh_token"],
		"token_endpoint_auth_methods_supported": [
			"client_secret_basic",
			"client_secret_post",
			"none",
		],
		"code_challenge_methods_supported": ["S256"],
		"claims_supported": [
			"sub",
			"name",
			"given_name",
			"family_name",
			"email",
			"email_verified",
		],
	})))
}

/// `GET /.well-known/jwks.json`: the public halves of the signing keys.
pub async fn jwks(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
) -> Result<Json<jsonwebtoken::jwk::JwkSet>, OAuthError> {
	super::enforce_rate_limit(&state, client_ip).await?;

	Ok(Json(
		keys::get_jwks(&state.config).map_err(OAuthError::server_error)?,
	))
}
