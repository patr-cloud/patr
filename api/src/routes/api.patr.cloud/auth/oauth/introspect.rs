use axum::{Form, Json, extract::State, http::HeaderMap};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use super::error::{OAuthError, OAuthErrorCode};
use crate::{
	models::{oauth, permissions},
	prelude::*,
	utils::extractors::ClientIP,
};

/// The form `/introspect` takes. `token_type_hint` is left for serde to drop,
/// since a token's prefix already says which kind it is.
#[derive(Debug, Deserialize)]
pub struct IntrospectRequest {
	/// The access or refresh token to describe.
	token: String,
	/// The client, when it authenticates in the form rather than with Basic.
	client_id: Option<String>,
	/// The client's secret, likewise.
	client_secret: Option<String>,
}

/// The introspection endpoint: `POST /auth/oauth/introspect` (RFC 7662).
/// Confidential clients only, since an open one lets anyone test tokens they
/// found (section 4). A client only hears about its own live tokens, and an
/// inactive response says nothing more (section 2.2).
pub async fn introspect(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	headers: HeaderMap,
	Form(body): Form<IntrospectRequest>,
) -> Result<Json<serde_json::Value>, OAuthError> {
	super::enforce_rate_limit(&state, client_ip).await?;

	let client_id = super::authenticate_client(
		&state,
		&headers,
		body.client_id.as_deref(),
		body.client_secret.as_deref(),
	)?;

	if state
		.config
		.oauth
		.clients
		.get(&client_id)
		.is_none_or(|client| client.client_secret.is_none())
	{
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidClient,
			"introspection requires a confidential client",
		));
	}

	let inactive = Json(serde_json::json!({ "active": false }));
	let mut database = state.database.acquire().await?;

	if body.token.starts_with(constants::REFRESH_TOKEN_PREFIX) &&
		permissions::has_valid_checksum(&body.token)
	{
		// A consumed token is inactive even within the grace window.
		let Some(row) = query!(
			r#"
			SELECT
				oauth_refresh_token.expiry,
				oauth_login.user_id,
				oauth_login.scope
			FROM
				oauth_refresh_token
			INNER JOIN
				oauth_login
			ON
				oauth_refresh_token.login_id = oauth_login.login_id
			WHERE
				oauth_refresh_token.token_hash = $1 AND
				oauth_refresh_token.consumed IS NULL AND
				oauth_refresh_token.expiry > $2 AND
				oauth_login.client_id = $3 AND
				oauth_login.revoked IS NULL;
			"#,
			hex::encode(Sha256::digest(&body.token)),
			OffsetDateTime::now_utc(),
			client_id,
		)
		.fetch_optional(&mut *database)
		.await?
		else {
			return Ok(inactive);
		};

		return Ok(Json(serde_json::json!({
			"active": true,
			"client_id": client_id,
			"scope": row.scope,
			"sub": row.user_id.to_string(),
			"exp": row.expiry.unix_timestamp(),
		})));
	}

	let Some(claims) = oauth::decode_access_token(&state.config, &body.token)
		.filter(|claims| claims.azp == client_id)
	else {
		return Ok(inactive);
	};

	let live = query!(
		r#"
		SELECT
			login_id
		FROM
			oauth_login
		WHERE
			login_id = $1 AND
			revoked IS NULL;
		"#,
		claims.sid,
	)
	.fetch_optional(&mut *database)
	.await?
	.is_some();

	if !live {
		return Ok(inactive);
	}

	Ok(Json(serde_json::json!({
		"active": true,
		"client_id": client_id,
		"scope": claims.scope,
		"token_type": "Bearer",
		"sub": claims.sub.to_string(),
		"aud": claims.aud,
		"iss": claims.iss,
		"exp": claims.exp.unix_timestamp(),
		"iat": claims.iat.unix_timestamp(),
		"nbf": claims.nbf.unix_timestamp(),
		"jti": claims.jti.to_string(),
	})))
}
