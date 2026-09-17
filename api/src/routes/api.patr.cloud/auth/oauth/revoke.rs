use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier as _, Version};
use axum::{
	Form,
	extract::State,
	http::{HeaderMap, StatusCode},
	response::{IntoResponse, Response},
};
use serde::Deserialize;

use super::{
	client_auth,
	error::{OAuthError, OAuthErrorCode},
};
use crate::{
	models::{oauth, permissions},
	prelude::*,
	utils::extractors::ClientIP,
};

/// The form body `/revoke` accepts.
#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
	/// The token being revoked. Either kind is accepted.
	token: String,
	/// The client's guess at which kind it is. RFC 7009 section 2.1 makes
	/// this a hint and nothing more: the server is expected to look in the
	/// other place if the hint turns out to be wrong.
	token_type_hint: Option<String>,
	/// `client_secret_post` credentials, when not sent as Basic.
	client_id: Option<String>,
	/// The client secret, for a confidential client authenticating by body.
	client_secret: Option<String>,
}

/// `POST /auth/oauth/revoke` — RFC 7009.
///
/// Revoking either kind of token takes the whole grant with it. RFC 7009
/// section 2.1 allows exactly that, and it is what a user means by "log this
/// app out": leaving the refresh token alive because they happened to hand
/// over the access token would be a surprise.
///
/// **An unknown token is a success.** The RFC is explicit, and the reason is
/// that the endpoint would otherwise be an oracle: a client could learn
/// whether a token it found is real by watching for a 400. Wrong client,
/// already revoked, expired, never existed — all of them are 200 with an
/// empty body. The only failures are a client that cannot authenticate and a
/// hint naming a token type this server does not issue.
pub async fn revoke(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	headers: HeaderMap,
	Form(body): Form<RevokeRequest>,
) -> Response {
	if let Err(error) = super::enforce_rate_limit(&state, client_ip).await {
		return error.into_response();
	}

	match handle(&state, &headers, body).await {
		Ok(()) => StatusCode::OK.into_response(),
		Err(err) => err.into_response(),
	}
}

/// The body of [`revoke`], separated so the error type can be rendered once.
async fn handle(
	state: &AppState,
	headers: &HeaderMap,
	body: RevokeRequest,
) -> Result<(), OAuthError> {
	let client_id = client_auth::authenticate_client(
		state,
		headers,
		body.client_id.as_deref(),
		body.client_secret.as_deref(),
	)?;

	// The hint is optional, but a hint for something we do not issue is a
	// real error rather than something to shrug at — RFC 7009 section 2.2.1.
	if let Some(hint) = body.token_type_hint.as_deref() &&
		!matches!(hint, "access_token" | "refresh_token")
	{
		return Err(OAuthError::new(
			OAuthErrorCode::UnsupportedTokenType,
			"`token_type_hint` must be `access_token` or `refresh_token`",
		));
	}

	let mut connection = state.database.begin().await?;
	let mut redis = state.redis.clone();

	// Resolve the token to the grant it belongs to, whichever kind it is.
	// A token that resolves to nothing, or to another client's grant, simply
	// falls through to the success below.
	let grant_id = resolve_grant(state, &mut connection, &client_id, &body.token).await?;

	if let Some(grant_id) = grant_id {
		oauth::revoke_grant(&mut connection, &mut redis, &grant_id)
			.await
			.map_err(OAuthError::server_error)?;
		connection.commit().await?;
	} else {
		debug!("Revocation request for a token that resolves to no live grant");
	}

	Ok(())
}

/// Finds the grant a presented token belongs to, if it belongs to one of this
/// client's.
///
/// Returns `None` for anything that is not this client's live token, which
/// the caller turns into a silent success. Deliberately does not distinguish
/// between the reasons — see [`revoke`].
async fn resolve_grant(
	state: &AppState,
	connection: &mut DatabaseTransaction,
	client_id: &str,
	token: &str,
) -> Result<Option<Uuid>, OAuthError> {
	// A refresh token is `patrv1.{secret}.{id}`; anything else is either a
	// JWT access token or noise.
	if let Ok((secret, token_id)) = client_auth::parse_refresh_token(token) {
		let Some(row) = query!(
			r#"
			SELECT
				oauth_refresh_token.token_hash,
				oauth_login.login_id AS "grant_id: Uuid",
				oauth_login.client_id
			FROM
				oauth_refresh_token
			INNER JOIN
				oauth_login
			ON
				oauth_refresh_token.login_id = oauth_login.login_id
			WHERE
				oauth_refresh_token.id = $1;
			"#,
			token_id as _,
		)
		.fetch_optional(&mut **connection)
		.await?
		else {
			return Ok(None);
		};

		// Another client's token is none of this client's business, and
		// saying so would confirm it exists.
		if row.client_id != client_id {
			warn!(
				"Client `{}` tried to revoke a token belonging to `{}`",
				client_id, row.client_id
			);
			return Ok(None);
		}

		// The secret still has to check out. Without this, knowing any token
		// id would be enough to kill the grant behind it.
		let verified = Argon2::new_with_secret(
			state.config.password_pepper.as_ref(),
			Algorithm::Argon2id,
			Version::V0x13,
			constants::HASHING_PARAMS,
		)
		.map_err(OAuthError::server_error)?
		.verify_password(
			secret.as_bytes(),
			&PasswordHash::new(&row.token_hash).map_err(OAuthError::server_error)?,
		)
		.is_ok();

		return Ok(verified.then_some(row.grant_id));
	}

	// Otherwise treat it as an access token. Its signature and audience have
	// to be valid — the `sid` of an unverified JWT is just an attacker's
	// choice of grant to destroy.
	let Ok(claims) = permissions::validate_access_token(&state.config, token) else {
		return Ok(None);
	};

	let Some(row) = query!(
		r#"
		SELECT
			client_id
		FROM
			oauth_login
		WHERE
			login_id = $1;
		"#,
		claims.sid as _,
	)
	.fetch_optional(&mut **connection)
	.await?
	else {
		return Ok(None);
	};

	if row.client_id != client_id {
		warn!(
			"Client `{}` tried to revoke an access token belonging to `{}`",
			client_id, row.client_id
		);
		return Ok(None);
	}

	Ok(Some(claims.sid))
}
