use axum::{
	Form,
	extract::State,
	http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::error::OAuthError;
use crate::{
	models::{oauth, permissions},
	prelude::*,
	utils::extractors::ClientIP,
};

/// The form `/revoke` takes. `token_type_hint` is left for serde to drop,
/// since a token's prefix already says which kind it is.
#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
	/// The access or refresh token to revoke.
	token: String,
	/// The client, when it authenticates in the form rather than with Basic.
	client_id: Option<String>,
	/// The client's secret, likewise.
	client_secret: Option<String>,
}

/// The revocation endpoint: `POST /auth/oauth/revoke` (RFC 7009). Either
/// token takes its whole grant with it. Anything that isn't one of the
/// client's tokens is still a 200 (section 2.2), so this can't be used to
/// check whether a token is real.
pub async fn revoke(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	headers: HeaderMap,
	Form(body): Form<RevokeRequest>,
) -> Result<StatusCode, OAuthError> {
	super::enforce_rate_limit(&state, client_ip).await?;

	let client_id = super::authenticate_client(
		&state,
		&headers,
		body.client_id.as_deref(),
		body.client_secret.as_deref(),
	)?;

	let mut connection = state.database.begin().await?;

	let login_id = if body.token.starts_with(constants::REFRESH_TOKEN_PREFIX) &&
		permissions::has_valid_checksum(&body.token)
	{
		query!(
			r#"
			SELECT
				oauth_login.login_id
			FROM
				oauth_refresh_token
			INNER JOIN
				oauth_login
			ON
				oauth_refresh_token.login_id = oauth_login.login_id
			WHERE
				oauth_refresh_token.token_hash = $1 AND
				oauth_login.client_id = $2;
			"#,
			hex::encode(Sha256::digest(&body.token)),
			client_id,
		)
		.fetch_optional(&mut *connection)
		.await?
		.map(|row| row.login_id)
	} else {
		oauth::decode_access_token(&state.config, &body.token)
			.filter(|claims| claims.azp == client_id)
			.map(|claims| claims.sid)
	};

	if let Some(login_id) = login_id {
		oauth::revoke_grant(&mut connection, &mut state.redis.clone(), &login_id)
			.await
			.map_err(OAuthError::server_error)?;
	}

	connection.commit().await?;

	Ok(StatusCode::OK)
}
