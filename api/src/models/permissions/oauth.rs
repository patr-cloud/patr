use std::net::IpAddr;

use jsonwebtoken::{Algorithm, Validation};
use models::RequestUserData;
use rustis::client::Client as RedisClient;
use time::OffsetDateTime;

use crate::{
	models::oauth::{api_audience, claims::OAuthAccessTokenClaims, issuer, keys},
	prelude::*,
	utils::config::AppConfig,
};

/// The `typ` header RFC 9068 gives an OAuth 2.0 access token.
///
/// Pinned so an id token — same algorithm, same signing key, but `typ: JWT`
/// and the *client* as its audience — cannot be presented here. Relying
/// parties treat id tokens as non-secret and write them to logs and cookies,
/// so this is the check that keeps one from becoming an API credential.
const ACCESS_TOKEN_TYP: &str = "at+jwt";

/// A validated OAuth access token: who it speaks for, and what they approved.
pub struct OAuthGrantContext {
	/// The user the grant acts on behalf of.
	pub user_data: RequestUserData,
	/// The identity scopes the user approved, space-delimited. Gates
	/// `/userinfo` and the id token; never API authority.
	pub scope: String,
	/// The client acting on the user's behalf.
	pub client_id: String,
}

pub(crate) async fn get_permissions(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	_client_ip: IpAddr,
	token: &str,
) -> Result<RequestUserData, ErrorType> {
	get_grant_context(database, redis, config, token)
		.await
		.map(|context| context.user_data)
}

/// Validates an OAuth access token and resolves the grant behind it.
///
/// Every request re-reads the `oauth_login` row rather than trusting the
/// token alone, exactly as the web dashboard re-reads `web_login`. That read
/// is what makes revocation instant: the token stays cryptographically valid
/// until it expires, but the grant it names is gone.
#[instrument(skip(database, redis, config, token))]
pub async fn get_grant_context(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	token: &str,
) -> Result<OAuthGrantContext, ErrorType> {
	trace!("Parsing authentication header as an OAuth access token");

	let header = jsonwebtoken::decode_header(token).map_err(|err| {
		warn!("Invalid OAuth access token header: {}", err);
		ErrorType::MalformedAccessToken
	})?;

	if header.typ.as_deref() != Some(ACCESS_TOKEN_TYP) {
		warn!(
			"OAuth access token has `typ` {:?}, expected `{}`",
			header.typ, ACCESS_TOKEN_TYP
		);
		return Err(ErrorType::MalformedAccessToken);
	}

	let kid = header.kid.as_deref().ok_or_else(|| {
		warn!("OAuth access token carries no `kid`");
		ErrorType::MalformedAccessToken
	})?;

	let Some(key) = keys::get_key_by_id(config, kid)? else {
		warn!("OAuth access token names an unknown `kid`: {}", kid);
		return Err(ErrorType::MalformedAccessToken);
	};

	// Algorithm, issuer and audience are all pinned in the validator rather
	// than checked afterwards, so there is no path where a token is decoded
	// without them having been enforced.
	let mut validation = Validation::new(Algorithm::ES256);
	validation.set_issuer(&[issuer(config)]);
	validation.set_audience(&[api_audience(config)]);
	validation.set_required_spec_claims(&["iss", "sub", "aud", "exp"]);

	let claims =
		jsonwebtoken::decode::<OAuthAccessTokenClaims>(token, &key.decoding_key()?, &validation)
			.map_err(|err| {
				warn!("Invalid OAuth access token: {}", err);
				ErrorType::AuthorizationTokenInvalid
			})?
			.claims;

	trace!("OAuth access token signature, issuer and audience are valid");

	if OffsetDateTime::now_utc() < claims.nbf {
		warn!("OAuth access token is not valid yet");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}

	let Some(grant) = query!(
		r#"
		SELECT
			oauth_login.client_id,
			oauth_login.scope,
			oauth_login.revoked,
			"user".*
		FROM
			oauth_login
		INNER JOIN
			"user"
		ON
			oauth_login.user_id = "user".id
		WHERE
			oauth_login.login_id = $1;
		"#,
		claims.sid as _,
	)
	.fetch_optional(&mut *database)
	.await?
	else {
		warn!("OAuth grant `{}` not found", claims.sid);
		return Err(ErrorType::AuthorizationTokenInvalid);
	};

	if grant
		.revoked
		.is_some_and(|revoked| revoked <= OffsetDateTime::now_utc())
	{
		warn!("OAuth grant `{}` has been revoked", claims.sid);
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("OAuth grant exists and is live");

	// `sub` names the user and `sid` names the grant. The lookup above
	// already pinned the grant, so this only fires on a hand-assembled
	// token — but it is what keeps the two claims from drifting apart.
	if claims.sub != grant.id.into() {
		warn!("OAuth access token `sub` does not match the user owning `sid`");
		return Err(ErrorType::MalformedAccessToken);
	}

	// The user's full current authority, computed fresh, with no ceiling
	// applied. Safe only because clients are first-party and declared in the
	// config; a third-party client could not ship without one.
	let permissions = super::get_current_permissions_for_user(
		&mut *database,
		redis,
		&claims.sid,
		&grant.id.into(),
	)
	.await?;

	Ok(OAuthGrantContext {
		user_data: RequestUserData::builder()
			.id(grant.id)
			.email(grant.email)
			.first_name(grant.first_name)
			.last_name(grant.last_name)
			.created(grant.created)
			.login_id(claims.sid)
			.permissions(permissions)
			.build(),
		scope: grant.scope,
		client_id: grant.client_id,
	})
}

/// Whether a bearer token looks like one this module should handle.
///
/// Classifies on the algorithm alone. Web dashboard JWTs are HS256 and API
/// tokens are not JWTs at all, so ES256 is unambiguous — and leaving `typ`
/// and `kid` to [`get_grant_context`] means an id token is rejected with a
/// precise error instead of being mistaken for a web session.
pub fn is_oauth_access_token(token: &str) -> bool {
	jsonwebtoken::decode_header(token).is_ok_and(|header| header.alg == Algorithm::ES256)
}
