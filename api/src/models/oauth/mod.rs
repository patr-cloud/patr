/// The claims carried by the tokens this provider issues.
pub mod claims;
/// The identity claims a grant's scopes let a client see.
pub mod identity;
/// The keys that sign those tokens.
pub mod keys;
/// The payloads the authorization code flow parks in Redis.
pub mod types;

use jsonwebtoken::{Algorithm, Validation};
use rustis::{client::Client as RedisClient, commands::GenericCommands};
use time::OffsetDateTime;

use self::claims::OAuthAccessTokenClaims;
use crate::{models::permissions, prelude::*, utils::config::AppConfig};

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

/// Decodes `token` as one of our OAuth access tokens, checking its signature,
/// `typ`, issuer, audience and expiry. `None` if it isn't one.
pub fn decode_access_token(config: &AppConfig, token: &str) -> Option<OAuthAccessTokenClaims> {
	let header = jsonwebtoken::decode_header(token).ok()?;
	// RFC 9068. An id token is signed with the same key, so this and the
	// audience are what keep one from being used here.
	if header.alg != Algorithm::ES256 || header.typ.as_deref() != Some("at+jwt") {
		return None;
	}

	let key = keys::get_key_by_id(config, header.kid.as_deref()?).ok()??;

	let mut validation = Validation::new(Algorithm::ES256);
	validation.set_issuer(&[issuer(config)]);
	validation.set_audience(&[issuer(config)]);
	validation.set_required_spec_claims(&["iss", "sub", "aud", "exp"]);
	validation.validate_nbf = true;

	jsonwebtoken::decode::<OAuthAccessTokenClaims>(token, &key.decoding_key().ok()?, &validation)
		.inspect_err(|err| warn!("Invalid OAuth access token: {err}"))
		.ok()
		.map(|data| data.claims)
}

/// Revokes a grant and every refresh token issued under it.
pub async fn revoke_grant(
	connection: &mut DatabaseConnection,
	redis: &mut RedisClient,
	login_id: &Uuid,
) -> Result<(), ErrorType> {
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
	.execute(&mut *connection)
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
	.fetch_all(&mut *connection)
	.await?;

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

	permissions::mark_login_stale(redis, login_id).await
}
