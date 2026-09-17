/// The claims carried by the tokens this provider issues.
pub mod claims;
/// The identity claims a grant's scopes entitle a client to see, shared by
/// `/userinfo` and the id token.
pub mod identity;
/// Signing keys and the JWKS.
pub mod keys;
/// The payloads parked in Redis between the front and back channels of the
/// authorization code flow.
pub mod types;

use time::OffsetDateTime;

use crate::{prelude::*, utils::config::AppConfig};

/// The OIDC issuer identifier.
///
/// Everything derives from this: the `iss` claim in every token, and the URL
/// a client fetches the discovery document from. Cloud serves the API on its
/// own subdomain; self-hosted path-routes it under `/api`.
pub fn issuer(config: &AppConfig) -> String {
	let base_domain = &config.server.base_domain;
	if cfg!(feature = "cloud") {
		format!("https://api.{base_domain}")
	} else {
		format!("https://{base_domain}/api")
	}
}

/// The audience an access token must name to be accepted by the API.
///
/// The same value as the issuer, because the API is both. What matters is
/// that it differs from an id token's audience — which is the client — so
/// the two cannot be swapped.
pub fn api_audience(config: &AppConfig) -> String {
	issuer(config)
}

/// Revokes a grant and everything issued under it: the same recipe
/// `logout.rs` uses for a web session, because a grant is a login. Shared by
/// reuse detection in `/token`, by `/revoke` and by the dashboard, so there is
/// one way a grant dies.
#[instrument(skip(connection, redis))]
pub async fn revoke_grant(
	connection: &mut DatabaseConnection,
	redis: &mut rustis::client::Client,
	login_id: &Uuid,
) -> Result<(), ErrorType> {
	use rustis::commands::{GenericCommands, StringCommands};

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
		login_id as _,
	)
	.execute(&mut *connection)
	.await?;

	// Every token in the family, so nothing outlives the revocation.
	let tokens = query!(
		r#"
		UPDATE
			oauth_refresh_token
		SET
			consumed = COALESCE(consumed, $1)
		WHERE
			login_id = $2
		RETURNING
			id AS "id: Uuid";
		"#,
		now,
		login_id as _,
	)
	.fetch_all(&mut *connection)
	.await?;

	// Otherwise a grant revoked mid-window would keep replaying a live pair
	// to anyone presenting the token that caused the revocation.
	if !tokens.is_empty() {
		redis
			.del(
				tokens
					.into_iter()
					.map(|token| redis::keys::oauth_replacement_tokens(&token.id))
					.collect::<Vec<_>>(),
			)
			.await?;
	}

	redis
		.del(redis::keys::permission_for_login_id(login_id))
		.await?;

	// Outlives the cache it invalidates, so a map written just before this
	// cannot survive the timestamp that condemns it.
	redis
		.setex(
			redis::keys::login_id_revocation_timestamp(login_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs() +
				100,
			now.unix_timestamp_nanos().to_string(),
		)
		.await?;

	Ok(())
}
