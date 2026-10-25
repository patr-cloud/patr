use std::collections::HashMap;

use axum::{
	Form,
	Json,
	extract::State,
	http::{
		HeaderMap,
		header::{CACHE_CONTROL, PRAGMA},
	},
	response::{IntoResponse, Response},
};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use headers::{Authorization, HeaderMapExt, authorization::Basic};
use jsonwebtoken::Header;
use rustis::commands::{GenericCommands, SetExpiration, StringCommands};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;

use super::error::{OAuthError, OAuthErrorCode};
use crate::{
	models::{
		oauth::{
			self,
			claims::OAuthAccessTokenClaims,
			keys,
			types::{OAuthAuthorizationCode, OAuthReplacementTokens},
		},
		permissions,
	},
	prelude::*,
	utils::extractors::ClientIP,
};

/// The form `/token` takes. One struct for both grants, since the spec puts
/// them in one flat form.
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
	/// Which grant is being exercised.
	grant_type: String,
	/// The client, when it authenticates in the form rather than with Basic.
	client_id: Option<String>,
	/// The client's secret, likewise.
	client_secret: Option<String>,
	/// `authorization_code`: the code being exchanged.
	code: Option<String>,
	/// `authorization_code`: the redirect URI the code was issued for.
	redirect_uri: Option<String>,
	/// `authorization_code`: the PKCE verifier.
	code_verifier: Option<String>,
	/// `refresh_token`: the token being rotated.
	refresh_token: Option<String>,
}

/// The only token type this server issues.
#[derive(Debug, Serialize)]
enum TokenType {
	/// RFC 6750.
	Bearer,
}

/// A successful token response (RFC 6749 section 5.1).
#[derive(Debug, Serialize)]
pub struct TokenResponse {
	/// The token to present to the API.
	access_token: String,
	/// Always `Bearer`.
	token_type: TokenType,
	/// Seconds until the access token expires.
	expires_in: u64,
	/// The new refresh token, when one was issued.
	#[serde(skip_serializing_if = "Option::is_none")]
	refresh_token: Option<String>,
	/// The granted scopes, space-delimited.
	scope: String,
}

impl TokenResponse {
	/// A response carrying a freshly minted access token.
	fn new(access_token: String, refresh_token: Option<String>, scope: String) -> Self {
		Self {
			access_token,
			token_type: TokenType::Bearer,
			expires_in: constants::ACCESS_TOKEN_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			refresh_token,
			scope,
		}
	}
}

/// The token endpoint: `POST /auth/oauth/token`.
#[instrument(skip(state, headers, body))]
pub async fn token(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	headers: HeaderMap,
	Form(body): Form<TokenRequest>,
) -> Response {
	if let Err(error) = super::enforce_rate_limit(&state, client_ip).await {
		return error.into_response();
	}

	match handle(&state, &headers, body).await {
		// RFC 6749 section 5.1: tokens must never be cached.
		Ok(response) => (
			[(CACHE_CONTROL, "no-store"), (PRAGMA, "no-cache")],
			Json(response),
		)
			.into_response(),
		Err(error) => error.into_response(),
	}
}

/// Authenticates the client and dispatches on the grant type.
async fn handle(
	state: &AppState,
	headers: &HeaderMap,
	body: TokenRequest,
) -> Result<TokenResponse, OAuthError> {
	let client_id = authenticate_client(state, headers, &body)?;

	match body.grant_type.as_str() {
		"authorization_code" => exchange_code(state, &client_id, body).await,
		"refresh_token" => rotate_refresh_token(state, &client_id, body).await,
		other => Err(OAuthError::new(
			OAuthErrorCode::UnsupportedGrantType,
			format!("`{other}` is not a grant type this server implements"),
		)),
	}
}

/// Authenticates the client and returns its id. Takes credentials over Basic
/// and in the form (RFC 6749 section 2.3.1), since Grafana's client tries
/// Basic first. A public client presents none, and relies on PKCE.
fn authenticate_client(
	state: &AppState,
	headers: &HeaderMap,
	body: &TokenRequest,
) -> Result<String, OAuthError> {
	// Both halves of Basic are form-encoded before being base64'd.
	let decode = |value: &str| {
		serde_qs::from_str::<HashMap<String, String>>(&format!("v={value}"))
			.ok()
			.and_then(|mut map| map.remove("v"))
			.unwrap_or_else(|| value.to_owned())
	};

	let (client_id, presented_secret) =
		match (headers.typed_get::<Authorization<Basic>>(), &body.client_id) {
			(Some(basic), _) => (decode(basic.username()), Some(decode(basic.password()))),
			(None, Some(client_id)) => (client_id.clone(), body.client_secret.clone()),
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
		// Compared by digest, so the time taken doesn't leak how much matched.
		(Some(expected), Some(presented))
			if Sha256::digest(expected) == Sha256::digest(&presented) => {}
		(None, None) => {}
		_ => {
			return Err(OAuthError::new(
				OAuthErrorCode::InvalidClient,
				"client authentication failed",
			));
		}
	}

	Ok(client_id)
}

/// Exchanges an authorization code for tokens, creating the grant.
async fn exchange_code(
	state: &AppState,
	client_id: &str,
	body: TokenRequest,
) -> Result<TokenResponse, OAuthError> {
	let code = body
		.code
		.as_deref()
		.ok_or_else(|| OAuthError::new(OAuthErrorCode::InvalidRequest, "`code` is required"))?;

	// GETDEL makes the code single-use.
	let issued = state
		.redis
		.clone()
		.getdel::<Option<String>>(redis::keys::oauth_authorization_code(code))
		.await?
		.as_deref()
		.and_then(|payload| serde_json::from_str::<OAuthAuthorizationCode>(payload).ok())
		.ok_or_else(|| {
			OAuthError::new(
				OAuthErrorCode::InvalidGrant,
				"the code is unknown, expired, or has already been used",
			)
		})?;

	if issued.client_id != client_id {
		warn!(
			"Client `{}` tried to exchange a code issued to `{}`",
			client_id, issued.client_id
		);
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the code was issued to a different client",
		));
	}

	if body.redirect_uri.as_deref() != Some(issued.redirect_uri.as_str()) {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"`redirect_uri` does not match the one the code was issued for",
		));
	}

	let verifier = body.code_verifier.as_deref().ok_or_else(|| {
		OAuthError::new(
			OAuthErrorCode::InvalidRequest,
			"`code_verifier` is required",
		)
	})?;

	// S256 only. Compared by digest, like the client secret.
	let computed = BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(verifier));
	if Sha256::digest(computed) != Sha256::digest(&issued.code_challenge) {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the code verifier does not match the challenge",
		));
	}

	let mut connection = state.database.begin().await?;
	let now = OffsetDateTime::now_utc();
	let scope = issued.scopes.join(" ");

	let login_id = query!(
		r#"
		WITH client AS (
			INSERT INTO
				actor_client(id, actor_client_type, actor_id)
			VALUES
				(GENERATE_LOGIN_ID(), 'user_login', $1)
			RETURNING id
		)
		INSERT INTO
			user_login(
				login_id,
				user_id,
				login_type,
				created
			)
		SELECT
			client.id,
			$1,
			'oauth_login',
			$2
		FROM
			client
		RETURNING user_login.login_id;
		"#,
		issued.user_id,
		now,
	)
	.fetch_one(&mut *connection)
	.await?
	.login_id;

	query!(
		r#"
		INSERT INTO
			oauth_login(
				login_id,
				user_id,
				client_id,
				scope,
				created,
				last_used,
				created_ip,
				created_user_agent
			)
		VALUES
			($1, $2, $3, $4, $5, $5, $6, $7);
		"#,
		login_id,
		issued.user_id,
		client_id,
		scope,
		now,
		IpNetwork::from(issued.created_ip),
		issued.created_user_agent,
	)
	.execute(&mut *connection)
	.await?;

	let access_token =
		mint_access_token(state, &issued.user_id, &login_id, client_id, &scope, now)?;

	// Without `offline_access` there's nothing to refresh with, and the grant
	// lasts as long as its access token.
	let refresh_token = if issued.scopes.iter().any(|scope| scope == "offline_access") {
		Some(issue_refresh_token(&mut connection, &login_id, now).await?)
	} else {
		None
	};

	connection.commit().await?;

	Ok(TokenResponse::new(access_token, refresh_token, scope))
}

/// Signs an ES256 access token for a grant.
fn mint_access_token(
	state: &AppState,
	user_id: &Uuid,
	login_id: &Uuid,
	client_id: &str,
	scope: &str,
	now: OffsetDateTime,
) -> Result<String, OAuthError> {
	let key = keys::get_signing_key(&state.config).map_err(OAuthError::server_error)?;

	let mut header = Header::new(jsonwebtoken::Algorithm::ES256);
	header.kid = Some(key.kid.clone());
	// RFC 9068. With the audience, this is what keeps an id token from being
	// accepted as an access token.
	header.typ = Some("at+jwt".to_owned());

	jsonwebtoken::encode(
		&header,
		&OAuthAccessTokenClaims {
			iss: oauth::issuer(&state.config),
			sub: *user_id,
			aud: oauth::issuer(&state.config),
			azp: client_id.to_owned(),
			sid: *login_id,
			scope: scope.to_owned(),
			exp: now + constants::ACCESS_TOKEN_VALIDITY,
			nbf: now,
			iat: now,
			jti: Uuid::now_v1(),
		},
		&key.encoding_key,
	)
	.map_err(OAuthError::server_error)
}

/// Issues a refresh token as the grant's live one. A partial unique index
/// allows one unconsumed token per grant, so the previous one must already be
/// marked consumed.
async fn issue_refresh_token(
	connection: &mut DatabaseTransaction,
	login_id: &Uuid,
	now: OffsetDateTime,
) -> Result<String, OAuthError> {
	let refresh_token = permissions::generate_refresh_token();

	query!(
		r#"
		INSERT INTO
			oauth_refresh_token(
				id,
				login_id,
				token_hash,
				created,
				expiry
			)
		VALUES
			($1, $2, $3, $4, $5);
		"#,
		Uuid::new_v4(),
		login_id,
		hex::encode(Sha256::digest(&refresh_token)),
		now,
		now + constants::INACTIVE_REFRESH_TOKEN_VALIDITY,
	)
	.execute(&mut **connection)
	.await?;

	Ok(refresh_token)
}

/// Rotates a refresh token. Each is single-use, and presenting a consumed one
/// revokes the grant (OAuth 2.1 section 4.3), except within the grace window,
/// where a client racing itself gets back the pair the token already minted.
async fn rotate_refresh_token(
	state: &AppState,
	client_id: &str,
	body: TokenRequest,
) -> Result<TokenResponse, OAuthError> {
	let presented = body.refresh_token.as_deref().ok_or_else(|| {
		OAuthError::new(
			OAuthErrorCode::InvalidRequest,
			"`refresh_token` is required",
		)
	})?;

	if !presented.starts_with(constants::REFRESH_TOKEN_PREFIX) ||
		!permissions::has_valid_checksum(presented)
	{
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the refresh token is malformed",
		));
	}

	let mut connection = state.database.begin().await?;
	let now = OffsetDateTime::now_utc();

	let token_hash = hex::encode(Sha256::digest(presented));

	// FOR UPDATE serialises concurrent rotations of the same token.
	let row = query!(
		r#"
		SELECT
			oauth_refresh_token.id,
			oauth_refresh_token.login_id,
			oauth_refresh_token.expiry,
			oauth_refresh_token.consumed,
			oauth_login.client_id,
			oauth_login.user_id,
			oauth_login.scope,
			oauth_login.revoked
		FROM
			oauth_refresh_token
		INNER JOIN
			oauth_login
		ON
			oauth_refresh_token.login_id = oauth_login.login_id
		WHERE
			oauth_refresh_token.token_hash = $1
		FOR UPDATE OF
			oauth_refresh_token;
		"#,
		token_hash,
	)
	.fetch_optional(&mut *connection)
	.await?
	.ok_or_else(|| OAuthError::new(OAuthErrorCode::InvalidGrant, "the refresh token is unknown"))?;

	if row.client_id != client_id {
		warn!(
			"Client `{}` presented a refresh token belonging to `{}`",
			client_id, row.client_id
		);
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the refresh token was issued to a different client",
		));
	}

	// Before the grace window, so a revoked grant can't keep replaying its
	// last pair.
	if row.revoked.is_some_and(|revoked| revoked <= now) {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the grant has been revoked",
		));
	}

	if let Some(consumed) = row.consumed {
		if now - consumed <= constants::OAUTH_REFRESH_TOKEN_GRACE_PERIOD {
			let replacement = state
				.redis
				.clone()
				.get::<Option<String>>(redis::keys::oauth_replacement_tokens(&token_hash))
				.await?
				.as_deref()
				.map(serde_json::from_str::<OAuthReplacementTokens>)
				.transpose()?;

			// The winner writes the key before its commit releases the row,
			// so a miss means Redis lost it. Refused, but not taken as a replay.
			let Some(replacement) = replacement else {
				warn!(
					"Replacement tokens missing from Redis for grant `{}`",
					row.login_id
				);
				return Err(OAuthError::new(
					OAuthErrorCode::InvalidGrant,
					"the refresh token has already been used",
				));
			};

			connection.commit().await?;

			return Ok(TokenResponse::new(
				replacement.access_token,
				Some(replacement.refresh_token),
				row.scope,
			));
		}

		warn!(
			"Refresh token replay detected for grant `{}`, revoking it",
			row.login_id
		);
		revoke_grant(state, &mut connection, &row.login_id).await?;
		connection.commit().await?;

		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the refresh token has already been used",
		));
	}

	if row.expiry <= now {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the refresh token has expired",
		));
	}

	let access_token = mint_access_token(
		state,
		&row.user_id,
		&row.login_id,
		client_id,
		&row.scope,
		now,
	)?;

	query!(
		r#"
		UPDATE
			oauth_refresh_token
		SET
			consumed = $1
		WHERE
			id = $2;
		"#,
		now,
		row.id,
	)
	.execute(&mut *connection)
	.await?;

	let refresh_token = issue_refresh_token(&mut connection, &row.login_id, now).await?;

	query!(
		r#"
		UPDATE
			oauth_login
		SET
			last_used = $1
		WHERE
			login_id = $2;
		"#,
		now,
		row.login_id,
	)
	.execute(&mut *connection)
	.await?;

	// Last before the commit, which releases the row: a racer can't see
	// `consumed` set until this key exists.
	state
		.redis
		.clone()
		.set_with_options(
			redis::keys::oauth_replacement_tokens(&token_hash),
			serde_json::to_string(&OAuthReplacementTokens {
				access_token: access_token.clone(),
				refresh_token: refresh_token.clone(),
			})?,
			None,
			SetExpiration::Ex(
				constants::OAUTH_REFRESH_TOKEN_GRACE_PERIOD
					.whole_seconds()
					.unsigned_abs(),
			),
		)
		.await?;

	connection.commit().await?;

	Ok(TokenResponse::new(
		access_token,
		Some(refresh_token),
		row.scope,
	))
}

/// Revokes a grant and every refresh token issued under it.
async fn revoke_grant(
	state: &AppState,
	connection: &mut DatabaseTransaction,
	login_id: &Uuid,
) -> Result<(), OAuthError> {
	let now = OffsetDateTime::now_utc();

	query!(
		r#"
		UPDATE
			oauth_login
		SET
			revoked = $1
		WHERE
			login_id = $2 AND
			revoked IS NULL;
		"#,
		now,
		login_id,
	)
	.execute(&mut **connection)
	.await?;

	let tokens = query!(
		r#"
		UPDATE
			oauth_refresh_token
		SET
			consumed = COALESCE(consumed, $1)
		WHERE
			login_id = $2
		RETURNING
			token_hash;
		"#,
		now,
		login_id,
	)
	.fetch_all(&mut **connection)
	.await?;

	let mut redis = state.redis.clone();

	// Or a grant revoked mid-window would keep replaying a live pair.
	if !tokens.is_empty() {
		redis
			.del(
				tokens
					.into_iter()
					.map(|token| redis::keys::oauth_replacement_tokens(&token.token_hash))
					.collect::<Vec<_>>(),
			)
			.await?;
	}

	permissions::mark_login_stale(&mut redis, login_id)
		.await
		.map_err(OAuthError::server_error)
}
