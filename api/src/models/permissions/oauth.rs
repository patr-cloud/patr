use jsonwebtoken::{Algorithm, Validation};
use time::OffsetDateTime;

use crate::{
	models::{
		oauth::{self, claims::OAuthAccessTokenClaims, keys},
		redis::{ActorAuthDataCache, ActorAuthDataCacheKind},
	},
	prelude::*,
	utils::config::AppConfig,
};

/// Decodes `token` as one of our OAuth access tokens, checking its signature,
/// `typ`, issuer, audience and expiry. `None` if it isn't one.
pub(super) fn decode_access_token(
	config: &AppConfig,
	token: &str,
) -> Option<OAuthAccessTokenClaims> {
	let header = jsonwebtoken::decode_header(token).ok()?;
	// RFC 9068. An id token is signed with the same key, so this and the
	// audience are what keep one from being used here.
	if header.alg != Algorithm::ES256 || header.typ.as_deref() != Some("at+jwt") {
		return None;
	}

	let key = keys::get_key_by_id(config, header.kid.as_deref()?).ok()??;

	let mut validation = Validation::new(Algorithm::ES256);
	validation.set_issuer(&[oauth::issuer(config)]);
	validation.set_audience(&[oauth::issuer(config)]);
	validation.set_required_spec_claims(&["iss", "sub", "aud", "exp"]);
	validation.validate_nbf = true;

	jsonwebtoken::decode::<OAuthAccessTokenClaims>(token, &key.decoding_key().ok()?, &validation)
		.inspect_err(|err| warn!("Invalid OAuth access token: {err}"))
		.ok()
		.map(|data| data.claims)
}

/// Load everything the cache holds for the OAuth grant `login_id`: the user
/// behind it and their permissions. A revoked grant has nothing to load.
pub(super) async fn load_actor_auth_data(
	database: &mut DatabaseConnection,
	login_id: &Uuid,
) -> Result<ActorAuthDataCache, ErrorType> {
	// Taken before the lookup, so a stamp written while the lookup is in
	// flight still marks this entry stale.
	let created_at = OffsetDateTime::now_utc();

	let Some(user) = query!(
		r#"
		SELECT
			"user".id,
			"user".email,
			"user".first_name,
			"user".last_name,
			"user".created
		FROM
			oauth_login
		INNER JOIN
			"user"
		ON
			"user".id = oauth_login.user_id
		WHERE
			oauth_login.login_id = $1 AND
			oauth_login.revoked IS NULL;
		"#,
		login_id
	)
	.fetch_optional(&mut *database)
	.await?
	else {
		warn!("OAuth grant not found");
		return Err(ErrorType::AuthorizationTokenInvalid);
	};

	let permissions = super::web_dashboard::load_user_permissions(&mut *database, &user.id).await?;

	Ok(ActorAuthDataCache {
		login_id: *login_id,
		actor_id: user.id,
		kind: ActorAuthDataCacheKind::OAuthLogin {
			email: user.email,
			first_name: user.first_name,
			last_name: user.last_name,
			created: user.created,
		},
		permissions,
		created_at,
	})
}
