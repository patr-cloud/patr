use argon2::{
	Algorithm,
	Argon2,
	PasswordHash,
	PasswordHasher as _,
	PasswordVerifier as _,
	Version,
	password_hash::generate_salt,
};
use axum::{
	Json,
	extract::State,
	http::HeaderMap,
	response::{IntoResponse, Response},
};
use base64::Engine;
use jsonwebtoken::Header;
use rustis::commands::StringCommands;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;

use super::{
	client_auth,
	error::{OAuthError, OAuthErrorCode},
};
use crate::{
	models::oauth::{
		self,
		claims::{IdTokenClaims, OAuthAccessTokenClaims},
		identity::{UserIdentity, build_identity_claims},
		keys,
		types::{OAuthAuthorizationCode, OAuthReplacementTokens, hash_code},
	},
	prelude::*,
	utils::{config::OAuthClientConfig, extractors::ClientIP},
};

/// The form body `/token` accepts.
///
/// One struct for both grant types, with the grant-specific fields optional,
/// because that is how the request arrives on the wire — the spec puts them
/// all in one flat form.
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
	/// Which grant is being exercised.
	grant_type: String,
	/// The client, when it authenticates by form field rather than Basic.
	client_id: Option<String>,
	/// The client secret, likewise.
	client_secret: Option<String>,
	/// `authorization_code`: the code being exchanged.
	code: Option<String>,
	/// `authorization_code`: the URI the code was issued for, which must
	/// match again.
	redirect_uri: Option<String>,
	/// `authorization_code`: the PKCE verifier.
	code_verifier: Option<String>,
	/// `refresh_token`: the token being rotated.
	refresh_token: Option<String>,
}

/// A successful token response, shaped exactly as RFC 6749 section 5.1
/// defines it.
///
/// Serialised bare, not wrapped in Patr's success envelope: a client library
/// reads these field names off the top level and would reject anything else.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
	/// The token to present to the API.
	access_token: String,
	/// Always `Bearer`.
	token_type: &'static str,
	/// Seconds until the access token expires.
	expires_in: u64,
	/// The rotated refresh token, when one was issued.
	#[serde(skip_serializing_if = "Option::is_none")]
	refresh_token: Option<String>,
	/// The id token, when `openid` was granted.
	#[serde(skip_serializing_if = "Option::is_none")]
	id_token: Option<String>,
	/// The granted scope, space-delimited.
	scope: String,
}

/// The token endpoint: `POST /auth/oauth/token`.
///
/// The back channel. A client's own server calls this — never a browser —
/// so it is the one place client credentials are presented.
#[instrument(skip(state, headers, body))]
pub async fn token(
	State(state): State<AppState>,
	ClientIP(client_ip): ClientIP,
	headers: HeaderMap,
	axum::extract::Form(body): axum::extract::Form<TokenRequest>,
) -> Response {
	// The endpoint most worth throttling on this whole surface: it takes a
	// client secret and a refresh token secret, and needs no session to
	// reach. Raw axum routes see none of the layer stack, so the check is
	// explicit.
	if let Err(error) = super::enforce_rate_limit(&state, client_ip).await {
		return error.into_response();
	}

	match handle(&state, &headers, body).await {
		Ok(response) => (
			// RFC 6749 section 5.1: tokens must not be cached, by anything,
			// ever.
			[
				(axum::http::header::CACHE_CONTROL, "no-store"),
				(axum::http::header::PRAGMA, "no-cache"),
			],
			Json(response),
		)
			.into_response(),
		Err(error) => error.into_response(),
	}
}

/// Dispatches on the grant type, once the client has authenticated.
async fn handle(
	state: &AppState,
	headers: &HeaderMap,
	body: TokenRequest,
) -> Result<TokenResponse, OAuthError> {
	let client_id = client_auth::authenticate_client(
		state,
		headers,
		body.client_id.as_deref(),
		body.client_secret.as_deref(),
	)?;
	let client = state
		.config
		.oauth
		.clients
		.get(&client_id)
		.ok_or_else(|| OAuthError::new(OAuthErrorCode::InvalidClient, "unknown client"))?;

	match body.grant_type.as_str() {
		"authorization_code" => exchange_code(state, &client_id, client, body).await,
		"refresh_token" => rotate_refresh_token(state, &client_id, body).await,
		other => Err(OAuthError::new(
			OAuthErrorCode::UnsupportedGrantType,
			format!("`{other}` is not a grant type this server implements"),
		)),
	}
}

/// Exchanges an authorization code for tokens, creating the grant.
async fn exchange_code(
	state: &AppState,
	client_id: &str,
	client: &OAuthClientConfig,
	body: TokenRequest,
) -> Result<TokenResponse, OAuthError> {
	let code = body
		.code
		.as_deref()
		.ok_or_else(|| OAuthError::new(OAuthErrorCode::InvalidRequest, "`code` is required"))?;

	let redis = state.redis.clone();

	// GETDEL: single-use, atomically. A second exchange finds nothing, which
	// is how a replayed code is caught.
	let parked: Option<String> = redis
		.getdel(redis::keys::oauth_authorization_code(&hash_code(code)))
		.await
		.map_err(|err| {
			error!("Error reading an authorization code from redis: {}", err);
			OAuthError::new(OAuthErrorCode::ServerError, "could not read the code")
		})?;

	let issued = parked
		.as_deref()
		.and_then(|payload| serde_json::from_str::<OAuthAuthorizationCode>(payload).ok())
		.ok_or_else(|| {
			OAuthError::new(
				OAuthErrorCode::InvalidGrant,
				"the code is unknown, expired, or has already been used",
			)
		})?;

	// The code is bound to one client. Without this, a client could exchange
	// a code issued to a different one.
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

	// RFC 6749 section 4.1.3: the redirect URI has to match the one the code
	// was issued for, so an attacker cannot swap the destination between
	// authorization and exchange.
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

	// The point of PKCE: only whoever started the flow knows the verifier,
	// so a stolen code is useless on its own.
	if !verify_pkce(verifier, &issued.code_challenge) {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the code verifier does not match the challenge",
		));
	}

	let mut connection = state.database.begin().await.map_err(server_error)?;
	let now = OffsetDateTime::now_utc();
	let scope = issued.scopes.join(" ");

	let login_id = create_grant(&mut connection, &issued, client_id, &scope, now).await?;

	let access_token =
		mint_access_token(state, &issued.user_id, &login_id, client_id, &scope, now)?;

	// `offline_access` is what a client asks for when it wants to keep
	// working once the user has gone away. Without it there is nothing to
	// refresh with, and the grant simply expires.
	let refresh_token = if issued.scopes.iter().any(|scope| scope == "offline_access") {
		Some(issue_refresh_token(state, &mut connection, &login_id, now).await?)
	} else {
		None
	};

	// `openid` is what turns an OAuth authorization into a login. `/authorize`
	// already refuses a request without it, so in practice this is always
	// taken — it is here so that relaxing that rule degrades to plain OAuth
	// rather than minting an id token nobody asked for.
	let id_token = if issued.scopes.iter().any(|scope| scope == "openid") {
		Some(
			mint_id_token(
				state,
				&mut connection,
				&issued,
				client_id,
				&scope,
				&access_token,
				now,
			)
			.await?,
		)
	} else {
		None
	};

	connection.commit().await.map_err(server_error)?;

	let _ = client;

	Ok(TokenResponse {
		access_token,
		token_type: "Bearer",
		expires_in: constants::ACCESS_TOKEN_VALIDITY
			.whole_seconds()
			.unsigned_abs(),
		refresh_token,
		id_token,
		scope,
	})
}

/// Mints the id token for a freshly created grant.
///
/// Only ever on the authorization code exchange, never on a refresh. OIDC
/// Core section 12.2 says a refresh response "might not contain an id token",
/// but that if it does and the original request carried a `nonce`, the same
/// value MUST be echoed — which would mean storing the nonce for the life of
/// the grant. A nonce exists to tie one id token to one authentication
/// request, so keeping it around to replay onto later ones is the wrong
/// shape. Clients that want a fresh id token can start a new authorization.
async fn mint_id_token(
	state: &AppState,
	connection: &mut DatabaseTransaction,
	issued: &OAuthAuthorizationCode,
	client_id: &str,
	scope: &str,
	access_token: &str,
	now: OffsetDateTime,
) -> Result<String, OAuthError> {
	let user = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			first_name,
			last_name,
			email
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		issued.user_id as _,
	)
	.fetch_one(&mut **connection)
	.await
	.map_err(server_error)?;

	let key = keys::get_signing_key(&state.config).map_err(|err| {
		error!("No signing key available: {}", err);
		OAuthError::new(OAuthErrorCode::ServerError, "no signing key is available")
	})?;

	let claims = IdTokenClaims {
		iss: oauth::issuer(&state.config),
		// The client, not the API — see the type's own note.
		aud: client_id.to_owned(),
		exp: now + constants::OAUTH_ID_TOKEN_VALIDITY,
		iat: now,
		// When the user last proved who they were, which is a property of the
		// browser session that approved this, not of this exchange.
		auth_time: issued.auth_time,
		nonce: issued.nonce.clone(),
		at_hash: at_hash(access_token),
		identity: build_identity_claims(
			&UserIdentity {
				id: user.id,
				first_name: &user.first_name,
				last_name: &user.last_name,
				email: &user.email,
			},
			scope,
		),
	};

	let mut header = Header::new(jsonwebtoken::Algorithm::ES256);
	header.kid = Some(key.kid.clone());

	jsonwebtoken::encode(&header, &claims, &key.encoding_key).map_err(|err| {
		error!("Error signing an id token: {}", err);
		OAuthError::new(OAuthErrorCode::ServerError, "could not sign the id token")
	})
}

/// The `at_hash` claim for an access token.
///
/// OIDC Core section 3.1.3.6: the left-most half of the hash the signing
/// algorithm implies, base64url-encoded. ES256 means SHA-256, so that is the
/// first 16 of its 32 bytes.
fn at_hash(access_token: &str) -> String {
	use base64::prelude::BASE64_URL_SAFE_NO_PAD;

	BASE64_URL_SAFE_NO_PAD.encode(&Sha256::digest(access_token.as_bytes())[..16])
}

/// Checks a PKCE verifier against the challenge the request was started with.
fn verify_pkce(verifier: &str, challenge: &str) -> bool {
	use base64::prelude::BASE64_URL_SAFE_NO_PAD;

	// S256 only — OAuth 2.1 removes `plain`, and `/authorize` refuses to
	// park a request with anything else.
	let computed = BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

	// Compared by digest, so a caller cannot learn how much of a guess was
	// right from how long the comparison took.
	Sha256::digest(computed.as_bytes()) == Sha256::digest(challenge.as_bytes())
}

/// Registers the grant as a `user_login` subtype, so it inherits audit, the
/// permission cache, and revocation.
async fn create_grant(
	connection: &mut DatabaseTransaction,
	issued: &OAuthAuthorizationCode,
	client_id: &str,
	scope: &str,
	now: OffsetDateTime,
) -> Result<Uuid, OAuthError> {
	let login_id: Uuid = query!(
		r#"
		WITH client AS (
			INSERT INTO
				actor_client(id, actor_client_type)
			VALUES
				(GENERATE_LOGIN_ID(), 'user_login')
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
		issued.user_id as _,
		now,
	)
	.fetch_one(&mut **connection)
	.await
	.map_err(server_error)?
	.login_id
	.into();

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
		login_id as _,
		issued.user_id as _,
		client_id,
		scope,
		now,
		IpNetwork::from(issued.created_ip),
		issued.created_user_agent,
	)
	.execute(&mut **connection)
	.await
	.map_err(server_error)?;

	Ok(login_id)
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
	let key = keys::get_signing_key(&state.config).map_err(|err| {
		error!("No signing key available: {}", err);
		OAuthError::new(OAuthErrorCode::ServerError, "no signing key is available")
	})?;

	let claims = OAuthAccessTokenClaims {
		iss: oauth::issuer(&state.config),
		// The user, not the grant. Stable across every grant they hold.
		sub: *user_id,
		// The API is the audience. Pinning this is what stops an id token —
		// which names the *client* as its audience — being replayed here as
		// an API credential.
		aud: oauth::api_audience(&state.config),
		azp: client_id.to_owned(),
		sid: *login_id,
		scope: scope.to_owned(),
		exp: now + constants::ACCESS_TOKEN_VALIDITY,
		nbf: now,
		iat: now,
		jti: Uuid::now_v1(),
	};

	let mut header = Header::new(jsonwebtoken::Algorithm::ES256);
	header.kid = Some(key.kid.clone());
	// RFC 9068. The other half of the defence above: even a token with the
	// right audience is refused by the API unless it was minted as an access
	// token.
	header.typ = Some("at+jwt".to_owned());

	jsonwebtoken::encode(&header, &claims, &key.encoding_key).map_err(|err| {
		error!("Error signing an access token: {}", err);
		OAuthError::new(OAuthErrorCode::ServerError, "could not sign the token")
	})
}

/// Issues a refresh token, which becomes the grant's live one.
///
/// A partial unique index enforces one unconsumed token per grant, so this
/// can only be called once the previous token has been marked consumed.
async fn issue_refresh_token(
	state: &AppState,
	connection: &mut DatabaseTransaction,
	login_id: &Uuid,
	now: OffsetDateTime,
) -> Result<String, OAuthError> {
	let token_id = Uuid::now_v1();
	let secret = Uuid::new_v4();

	let hashed = Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(server_error)?
	.hash_password_with_salt(secret.to_string().as_bytes(), &generate_salt())
	.map(|hash| hash.to_string())
	.map_err(|err| {
		error!("Error hashing a refresh token: {}", err);
		OAuthError::new(
			OAuthErrorCode::ServerError,
			"could not issue a refresh token",
		)
	})?;

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
		token_id as _,
		login_id as _,
		hashed,
		now,
		now + constants::INACTIVE_REFRESH_TOKEN_VALIDITY,
	)
	.execute(&mut **connection)
	.await
	.map_err(server_error)?;

	// Same shape as every other Patr credential: an id naming the row, and a
	// secret that is only ever stored hashed.
	Ok(format!("patrv1.{secret}.{token_id}"))
}

/// Turns any database failure into an opaque `server_error`.
fn server_error<E: std::fmt::Display>(err: E) -> OAuthError {
	error!("Database error in the token endpoint: {}", err);
	OAuthError::new(OAuthErrorCode::ServerError, "internal error")
}

/// Rotates a refresh token, detecting replay.
///
/// The shape OAuth 2.1 section 4.3 and RFC 9700 section 4.14 require for
/// public clients: each token is single-use, and presenting a consumed one
/// is treated as evidence the token was stolen — the whole grant goes.
///
/// The grace window is the part that is not in the spec but is in every
/// implementation of it. Clients fire concurrent requests; two of them 401
/// at the same moment, both refresh with the same token, and the second is
/// indistinguishable from an attack. Without the window, ordinary
/// concurrency logs the user out.
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

	let (secret, token_id) = client_auth::parse_refresh_token(presented)?;

	let mut connection = state.database.begin().await.map_err(server_error)?;
	let now = OffsetDateTime::now_utc();

	// FOR UPDATE serialises concurrent rotations of the same token. Without
	// it both callers verify before either writes, and both mint a child.
	let row = query!(
		r#"
		SELECT
			oauth_refresh_token.login_id AS "login_id: Uuid",
			oauth_refresh_token.token_hash,
			oauth_refresh_token.expiry,
			oauth_refresh_token.consumed,
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
			oauth_refresh_token.id = $1
		FOR UPDATE OF
			oauth_refresh_token;
		"#,
		token_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
	.map_err(server_error)?
	.ok_or_else(|| OAuthError::new(OAuthErrorCode::InvalidGrant, "the refresh token is unknown"))?;

	// A refresh token belongs to the client it was issued to.
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

	// Verified before anything else is decided. A wrong secret against a
	// real token id is a forgery attempt, not a replay — revoking the grant
	// for it would let anyone kill a victim's session by guessing token ids.
	let verified = Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(server_error)?
	.verify_password(
		secret.as_bytes(),
		&PasswordHash::new(&row.token_hash).map_err(server_error)?,
	)
	.is_ok();

	if !verified {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the refresh token is invalid",
		));
	}

	// Checked before the replay logic below, so that a revoked grant cannot
	// keep replaying its last pair for the rest of the grace window.
	if row.revoked.is_some_and(|revoked| revoked <= now) {
		return Err(OAuthError::new(
			OAuthErrorCode::InvalidGrant,
			"the grant has been revoked",
		));
	}

	if let Some(consumed) = row.consumed {
		// Already used. Either this client is racing itself, or the token
		// leaked and someone else got there first. Inside the grace window
		// the benign reading is by far the likelier one, so the pair this
		// token originally minted gets replayed instead.
		if now - consumed <= constants::OAUTH_REFRESH_TOKEN_GRACE_PERIOD {
			let parked: Option<String> = state
				.redis
				.clone()
				.get(redis::keys::oauth_replacement_tokens(&token_id))
				.await
				.map_err(server_error)?;

			let replacement = parked
				.as_deref()
				.map(serde_json::from_str::<OAuthReplacementTokens>)
				.transpose()
				.map_err(server_error)?;

			let Some(replacement) = replacement else {
				// The winner writes Redis while still holding this row's
				// `FOR UPDATE` lock, and only releases it by committing the
				// `consumed` write — so a racer can never arrive between the
				// two. Reaching here means Redis lost the key, not that the
				// token is being replayed, and taking the grant down over a
				// Redis hiccup would log the user out for an infrastructure
				// problem. Refuse the one request and leave the grant alone.
				warn!(
					"Replacement tokens missing from Redis for grant `{}`; refusing the \
					 refresh without revoking",
					row.login_id
				);
				return Err(OAuthError::new(
					OAuthErrorCode::InvalidGrant,
					"the refresh token has already been used",
				));
			};

			// Hand back exactly what this token minted the first time,
			// rather than issuing a second pair. Both racers end up holding
			// the same credentials, which is what they would have had if the
			// requests had been serialised.
			debug!("Replaying the replacement pair for a raced refresh token");
			connection.commit().await.map_err(server_error)?;

			return Ok(TokenResponse {
				access_token: replacement.access_token,
				token_type: "Bearer",
				expires_in: constants::ACCESS_TOKEN_VALIDITY
					.whole_seconds()
					.unsigned_abs(),
				refresh_token: Some(replacement.refresh_token),
				id_token: None,
				scope: row.scope,
			});
		}

		// Outside the window, so this is a replay. Revoke the family.
		warn!(
			"Refresh token replay detected for grant `{}`, revoking it",
			row.login_id
		);
		oauth::revoke_grant(&mut connection, &mut state.redis.clone(), &row.login_id)
			.await
			.map_err(server_error)?;
		connection.commit().await.map_err(server_error)?;

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

	// Before the new token is inserted, not after: a partial unique index
	// allows a grant only one unconsumed token, so inserting first would
	// collide with the row being replaced.
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
		token_id as _,
	)
	.execute(&mut *connection)
	.await
	.map_err(server_error)?;

	let refresh_token = issue_refresh_token(state, &mut connection, &row.login_id, now).await?;

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
		row.login_id as _,
	)
	.execute(&mut *connection)
	.await
	.map_err(server_error)?;

	// Deliberately the last thing before the commit. The `FOR UPDATE` lock
	// taken at the top of this function is still held, and it is the commit
	// that releases it — so a racing sibling cannot observe `consumed` until
	// after this key exists, and its grace path can rely on finding it. Being
	// last also means everything before the commit that can fail already has;
	// should the commit itself fail, nothing ever sees `consumed` set, so the
	// key is unreachable and the TTL clears it.
	let replacement = serde_json::to_string(&OAuthReplacementTokens {
		access_token: access_token.clone(),
		refresh_token: refresh_token.clone(),
	})
	.map_err(server_error)?;

	state
		.redis
		.clone()
		.setex(
			redis::keys::oauth_replacement_tokens(&token_id),
			constants::OAUTH_REFRESH_TOKEN_GRACE_PERIOD
				.whole_seconds()
				.unsigned_abs(),
			replacement,
		)
		.await
		.map_err(server_error)?;

	connection.commit().await.map_err(server_error)?;

	Ok(TokenResponse {
		access_token,
		token_type: "Bearer",
		expires_in: constants::ACCESS_TOKEN_VALIDITY
			.whole_seconds()
			.unsigned_abs(),
		refresh_token: Some(refresh_token),
		id_token: None,
		scope: row.scope,
	})
}
