use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;

use super::error::{OAuthError, OAuthErrorCode};
use crate::{
	models::oauth::{self, keys, types::SUPPORTED_SCOPES},
	prelude::*,
	utils::extractors::ClientIP,
};

/// The OIDC provider metadata document.
///
/// Only what this provider actually implements is advertised. A field listed
/// here is a promise a client library will hold us to — `openid-client` will
/// happily POST to a `revocation_endpoint` that does not exist — so anything
/// not yet mounted is simply absent rather than declared and broken.
#[derive(Debug, Serialize)]
struct ProviderMetadata {
	/// The issuer identifier. Every token's `iss`, and the prefix a client
	/// derives this document's own URL from.
	issuer: String,
	/// Where the browser is sent to start a flow.
	authorization_endpoint: String,
	/// Where a code is exchanged and a refresh token rotated.
	token_endpoint: String,
	/// Where the identity claims are read from.
	userinfo_endpoint: String,
	/// Where a client ends a grant it holds.
	revocation_endpoint: String,
	/// Where a confidential client asks whether a token is still good.
	introspection_endpoint: String,
	/// Where the public halves of the signing keys are published.
	jwks_uri: String,
	/// Authorization code only. OAuth 2.1 removes the implicit and password
	/// grants, and this provider never had them.
	response_types_supported: [&'static str; 1],
	/// A `sub` that is the same for every client. Pairwise identifiers would
	/// mean a per-client mapping table, and with first-party clients there is
	/// nobody to hide the user's identity from.
	subject_types_supported: [&'static str; 1],
	/// ES256, matching the keys in the JWKS.
	id_token_signing_alg_values_supported: [&'static str; 1],
	/// The scopes a client may ask for.
	scopes_supported: [&'static str; 4],
	/// Refresh is here because `offline_access` grants one; the rest of OAuth
	/// 2.1's grant types are deliberately not implemented.
	grant_types_supported: [&'static str; 2],
	/// Both forms of client authentication `/token` accepts. Grafana's client
	/// tries Basic first and falls back to the body, so both are advertised.
	token_endpoint_auth_methods_supported: [&'static str; 2],
	/// S256 only. OAuth 2.1 forbids `plain`, and `/authorize` refuses it.
	code_challenge_methods_supported: [&'static str; 1],
	/// Every claim `/userinfo` and the id token can carry, so a client knows
	/// what asking for `profile` or `email` will actually get it.
	claims_supported: [&'static str; 6],
}

/// `GET /.well-known/openid-configuration`.
///
/// Mounted relative to the API's own root rather than the server's, so the
/// path a client fetches is exactly `{issuer}/.well-known/…` in both flavours
/// — `https://api.<domain>` on cloud, `https://<domain>/api` self-hosted.
pub async fn openid_configuration(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
) -> Result<impl IntoResponse, OAuthError> {
	super::enforce_rate_limit(&state, client_ip).await?;

	let issuer = oauth::issuer(&state.config);

	Ok(Json(ProviderMetadata {
		authorization_endpoint: format!("{issuer}/auth/oauth/authorize"),
		token_endpoint: format!("{issuer}/auth/oauth/token"),
		userinfo_endpoint: format!("{issuer}/auth/oauth/userinfo"),
		revocation_endpoint: format!("{issuer}/auth/oauth/revoke"),
		introspection_endpoint: format!("{issuer}/auth/oauth/introspect"),
		jwks_uri: format!("{issuer}/.well-known/jwks.json"),
		issuer,
		response_types_supported: ["code"],
		subject_types_supported: ["public"],
		id_token_signing_alg_values_supported: ["ES256"],
		scopes_supported: SUPPORTED_SCOPES,
		grant_types_supported: ["authorization_code", "refresh_token"],
		token_endpoint_auth_methods_supported: ["client_secret_basic", "client_secret_post"],
		code_challenge_methods_supported: ["S256"],
		claims_supported: [
			"sub",
			"name",
			"given_name",
			"family_name",
			"email",
			"email_verified",
		],
	}))
}

/// `GET /.well-known/jwks.json` — the public halves of the signing keys.
///
/// Public by definition: this is what a relying party verifies id tokens
/// against, so it is unauthenticated and cacheable. Keys are published here
/// before they start signing, so a client with a cached copy has already seen
/// the next one by the time a token arrives naming it.
pub async fn jwks(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
) -> Result<impl IntoResponse, OAuthError> {
	// Cacheable and public, but not free: each call decodes every configured
	// key to derive its public half, so an unthrottled caller can force that
	// repeatedly.
	super::enforce_rate_limit(&state, client_ip).await?;

	let jwks = keys::get_jwks(&state.config).map_err(|err| {
		error!("Error building the JWKS: {}", err);
		OAuthError::new(OAuthErrorCode::ServerError, "could not read the keys")
	})?;

	Ok(Json(jwks))
}
