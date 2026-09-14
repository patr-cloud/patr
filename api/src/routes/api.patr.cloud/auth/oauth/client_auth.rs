use std::collections::HashMap;

use axum::http::HeaderMap;
use base64::{Engine as _, prelude::BASE64_STANDARD};
use sha2::{Digest, Sha256};

use super::error::{OAuthError, OAuthErrorCode};
use crate::prelude::*;

/// Authenticates the client and returns its id.
///
/// Both forms RFC 6749 section 2.3.1 defines are accepted, because Grafana's
/// `golang.org/x/oauth2` tries Basic first and falls back to form fields —
/// supporting only one of them would fail against it on the first request.
/// Public clients present no secret at all and are held up by PKCE instead.
pub fn authenticate_client(
	state: &AppState,
	headers: &HeaderMap,
	body_client_id: Option<&str>,
	body_client_secret: Option<&str>,
) -> Result<String, OAuthError> {
	let basic = parse_basic_auth(headers);

	let (client_id, presented_secret) = match (basic, body_client_id) {
		(Some((id, secret)), _) => (id, Some(secret)),
		(None, Some(id)) => (id.to_owned(), body_client_secret.map(ToOwned::to_owned)),
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
		// Confidential client: the secret must match.
		(Some(expected), Some(presented)) => {
			// Compared by digest so the comparison time does not depend on
			// how many leading characters happened to be right.
			if Sha256::digest(expected.as_bytes()) != Sha256::digest(presented.as_bytes()) {
				return Err(OAuthError::new(
					OAuthErrorCode::InvalidClient,
					"client authentication failed",
				));
			}
		}
		(Some(_), None) => {
			return Err(OAuthError::new(
				OAuthErrorCode::InvalidClient,
				"this client must authenticate with its secret",
			));
		}
		// Public client: no secret is registered, so none may be presented.
		// Accepting one would let a caller guess at a secret that does not
		// exist and be told it was right.
		(None, Some(_)) => {
			return Err(OAuthError::new(
				OAuthErrorCode::InvalidClient,
				"this client is public and must not present a secret",
			));
		}
		(None, None) => {}
	}

	Ok(client_id)
}

/// Reads `client_secret_basic` credentials from the `Authorization` header.
fn parse_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
	let encoded = headers
		.get(axum::http::header::AUTHORIZATION)?
		.to_str()
		.ok()?
		.strip_prefix("Basic ")?;

	let decoded = String::from_utf8(BASE64_STANDARD.decode(encoded).ok()?).ok()?;
	let (id, secret) = decoded.split_once(':')?;

	// RFC 6749 section 2.3.1 requires both halves to be form-urlencoded
	// before being base64'd, so a secret containing `:` or `%` survives.
	let decode = |value: &str| percent_decode(value).unwrap_or_else(|| value.to_owned());

	Some((decode(id), decode(secret)))
}

/// Splits a `patrv1.{secret}.{token_id}` refresh token.
pub fn parse_refresh_token(token: &str) -> Result<(String, Uuid), OAuthError> {
	let malformed = || {
		OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the refresh token is malformed",
		)
	};

	let (secret, token_id) = token
		.strip_prefix("patrv1.")
		.ok_or_else(malformed)?
		.split_once('.')
		.ok_or_else(malformed)?;

	Ok((
		secret.to_owned(),
		Uuid::parse_str(token_id).map_err(|_| malformed())?,
	))
}

/// Reverses `application/x-www-form-urlencoded` escaping for one value.
fn percent_decode(value: &str) -> Option<String> {
	serde_qs::from_str::<HashMap<String, String>>(&format!("v={value}"))
		.ok()?
		.remove("v")
}
