use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier as _, Version};
use axum::{
	Form,
	Json,
	extract::State,
	http::HeaderMap,
	response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::{
	client_auth,
	error::{OAuthError, OAuthErrorCode},
};
use crate::{
	models::{oauth, permissions},
	prelude::*,
	utils::extractors::ClientIP,
};

/// The form body `/introspect` accepts.
#[derive(Debug, Deserialize)]
pub struct IntrospectRequest {
	/// The token being asked about.
	token: String,
	/// Which kind the caller believes it is. Advisory, as in RFC 7009.
	token_type_hint: Option<String>,
	/// `client_secret_post` credentials, when not sent as Basic.
	client_id: Option<String>,
	/// The client secret.
	client_secret: Option<String>,
}

/// The introspection response of RFC 7662 section 2.2.
///
/// Every field but `active` is absent when the token is not active, and the
/// spec requires exactly that: an inactive response says nothing at all about
/// why, so the endpoint cannot be used to tell "expired" from "never existed".
#[derive(Debug, Serialize)]
struct IntrospectResponse {
	/// Whether the token is currently usable. The only field a caller can
	/// rely on being present.
	active: bool,
	/// The granted scope, space-delimited.
	#[serde(skip_serializing_if = "Option::is_none")]
	scope: Option<String>,
	/// The client the token was issued to.
	#[serde(skip_serializing_if = "Option::is_none")]
	client_id: Option<String>,
	/// Always `Bearer` for an access token; absent for a refresh token.
	#[serde(skip_serializing_if = "Option::is_none")]
	token_type: Option<&'static str>,
	/// The user the token acts for.
	#[serde(skip_serializing_if = "Option::is_none")]
	sub: Option<String>,
	/// When it expires, as unix seconds.
	#[serde(skip_serializing_if = "Option::is_none")]
	exp: Option<i64>,
	/// When it was issued, as unix seconds.
	#[serde(skip_serializing_if = "Option::is_none")]
	iat: Option<i64>,
	/// Not valid before this, as unix seconds.
	#[serde(skip_serializing_if = "Option::is_none")]
	nbf: Option<i64>,
	/// The audience — the API, for an access token.
	#[serde(skip_serializing_if = "Option::is_none")]
	aud: Option<String>,
	/// The issuer.
	#[serde(skip_serializing_if = "Option::is_none")]
	iss: Option<String>,
	/// The token's unique id.
	#[serde(skip_serializing_if = "Option::is_none")]
	jti: Option<String>,
}

impl IntrospectResponse {
	/// The response for anything that is not a live token of the calling
	/// client's.
	fn inactive() -> Self {
		Self {
			active: false,
			scope: None,
			client_id: None,
			token_type: None,
			sub: None,
			exp: None,
			iat: None,
			nbf: None,
			aud: None,
			iss: None,
			jti: None,
		}
	}
}

/// `POST /auth/oauth/introspect` — RFC 7662.
///
/// **Confidential clients only.** RFC 7662 section 4 names token fishing as
/// the attack this endpoint invites: an unauthenticated one lets anyone test
/// stolen or guessed tokens for validity. Requiring a secret narrows that to
/// callers we already trust, and every client with a reason to introspect —
/// a resource server checking a token it was handed — has one.
pub async fn introspect(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	headers: HeaderMap,
	Form(body): Form<IntrospectRequest>,
) -> Response {
	if let Err(error) = super::enforce_rate_limit(&state, client_ip).await {
		return error.into_response();
	}

	match handle(&state, &headers, body).await {
		Ok(response) => Json(response).into_response(),
		Err(err) => err.into_response(),
	}
}

/// The body of [`introspect`], separated so the error type renders once.
async fn handle(
	state: &AppState,
	headers: &HeaderMap,
	body: IntrospectRequest,
) -> Result<IntrospectResponse, OAuthError> {
	let client_id = client_auth::authenticate_client(
		state,
		headers,
		body.client_id.as_deref(),
		body.client_secret.as_deref(),
	)?;

	// A public client has no secret, so authenticating one proves nothing.
	// Letting it through would leave the fishing hole wide open.
	let is_confidential = state
		.config
		.oauth
		.clients
		.get(&client_id)
		.is_some_and(|client| client.client_secret.is_some());

	if !is_confidential {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidClient,
			"introspection requires a confidential client",
		));
	}

	let _ = body.token_type_hint;

	let mut connection = state.database.begin().await?;
	let response = introspect_token(state, &mut connection, &client_id, &body.token).await?;
	connection.commit().await?;

	Ok(response)
}

/// Works out what, if anything, to say about a presented token.
async fn introspect_token(
	state: &AppState,
	connection: &mut DatabaseTransaction,
	client_id: &str,
	token: &str,
) -> Result<IntrospectResponse, OAuthError> {
	let now = OffsetDateTime::now_utc();

	if let Ok((secret, token_id)) = client_auth::parse_refresh_token(token) {
		let Some(row) = query!(
			r#"
			SELECT
				oauth_refresh_token.token_hash,
				oauth_refresh_token.expiry,
				oauth_refresh_token.consumed,
				oauth_login.login_id AS "grant_id: Uuid",
				oauth_login.client_id,
				oauth_login.user_id AS "user_id: Uuid",
				oauth_login.scope,
				oauth_login.revoked
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
			return Ok(IntrospectResponse::inactive());
		};

		if row.client_id != client_id {
			return Ok(IntrospectResponse::inactive());
		}

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

		// Consumed counts as inactive even inside the grace window: the
		// window exists so a raced client still gets a working pair, not so
		// that a spent token reports itself usable.
		let active = verified &&
			row.consumed.is_none() &&
			row.expiry > now &&
			!row.revoked.is_some_and(|revoked| revoked <= now);

		if !active {
			return Ok(IntrospectResponse::inactive());
		}

		return Ok(IntrospectResponse {
			active: true,
			scope: Some(row.scope),
			client_id: Some(row.client_id),
			sub: Some(row.user_id.to_string()),
			exp: Some(row.expiry.unix_timestamp()),
			..IntrospectResponse::inactive()
		});
	}

	// Otherwise, an access token.
	let Ok(claims) = permissions::validate_access_token(&state.config, token) else {
		return Ok(IntrospectResponse::inactive());
	};

	let Some(row) = query!(
		r#"
		SELECT
			client_id,
			revoked
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
		return Ok(IntrospectResponse::inactive());
	};

	if row.client_id != client_id || row.revoked.is_some_and(|revoked| revoked <= now) {
		return Ok(IntrospectResponse::inactive());
	}

	Ok(IntrospectResponse {
		active: true,
		scope: Some(claims.scope),
		client_id: Some(row.client_id),
		token_type: Some("Bearer"),
		sub: Some(claims.sub.to_string()),
		exp: Some(claims.exp.unix_timestamp()),
		iat: Some(claims.iat.unix_timestamp()),
		nbf: Some(claims.nbf.unix_timestamp()),
		aud: Some(claims.aud),
		iss: Some(oauth::issuer(&state.config)),
		jti: Some(claims.jti.to_string()),
	})
}
