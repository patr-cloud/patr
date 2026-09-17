use std::{net::IpAddr, ops::Sub};

use jsonwebtoken::{DecodingKey, TokenData, Validation};
use models::RequestUserData;
use rustis::client::Client as RedisClient;
use time::OffsetDateTime;

use crate::{models::access_token_data::AccessTokenData, prelude::*, utils::config::AppConfig};

pub(crate) async fn get_permissions(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	_client_ip: IpAddr,
	token: &str,
) -> Result<RequestUserData, ErrorType> {
	trace!("Parsing authentication header as a JWT");

	let TokenData {
		header: _,
		claims:
			AccessTokenData {
				iss,
				sub,
				sid,
				aud,
				exp,
				nbf,
				iat: _,
				jti,
			},
	} = jsonwebtoken::decode(
		token,
		&DecodingKey::from_secret(config.jwt_secret.as_ref()),
		&{
			let mut validation = Validation::default();

			// We'll manually do this
			validation.validate_exp = false;
			validation.validate_nbf = false;
			validation.validate_aud = false;

			validation
		},
	)
	.map_err(|err| {
		warn!("Invalid JWT provided: {}", err);
		ErrorType::MalformedAccessToken
	})?;
	trace!("Authentication header is a valid JWT");

	if iss != constants::JWT_ISSUER {
		warn!("Invalid JWT issuer: {}", iss);
		return Err(ErrorType::MalformedAccessToken);
	}
	trace!("JWT issuer valid");

	// The token should have been issued within the last `REFRESH_TOKEN_VALIDITY`
	// duration
	if OffsetDateTime::now_utc().sub(jti.get_timestamp().ok_or(ErrorType::MalformedAccessToken)?) >
		AccessTokenData::REFRESH_TOKEN_VALIDITY
	{
		warn!("JWT is too old");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT JTI valid");

	if OffsetDateTime::now_utc() < nbf {
		warn!("JWT is not valid yet");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT NBF valid");

	if OffsetDateTime::now_utc() > exp {
		warn!("JWT has expired");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	trace!("JWT EXP valid");

	let Some(user) = query! {
		r#"
		SELECT
			"user".*
		FROM
			"user"
		INNER JOIN
			user_login
		ON
			"user".id = user_login.user_id
		INNER JOIN
			web_login
		ON
			user_login.login_id = web_login.login_id
		WHERE
			user_login.login_id = $1 AND
			user_login.login_type = 'web_login';
		"#,
		sid as _
	}
	.fetch_optional(&mut *database)
	.await?
	else {
		warn!("web login not found");
		// No specific error for API token not found, since we don't want to leak
		// information about whether a loginId is valid or if it's expired
		return Err(ErrorType::AuthorizationTokenInvalid);
	};
	trace!("Web login exists in the database");

	// `sub` names the user and `sid` names the session, so a token whose two
	// halves disagree is malformed regardless of how it got that way. The
	// lookup above already pinned the session, so this only ever fires on a
	// hand-assembled token — but it is the check that keeps the two claims
	// from silently drifting apart.
	if sub != user.id.into() {
		warn!("JWT `sub` does not match the user owning `sid`");
		return Err(ErrorType::MalformedAccessToken);
	}
	trace!("JWT sub matches the session's user");

	// Note: `web_login.token_expiry` is the refresh token's lifetime, not the
	// access token's. Access token validity is gated by the JWT's own `exp`
	// claim (checked above). Re-checking `token_expiry` here would prevent a
	// fresh JWT (post-refresh) from authenticating until the entire session
	// is renewed, and would also keep an old, expired JWT alive as long as
	// the session itself was still fresh. Both are wrong.

	if !aud
		.clone()
		.into_iter()
		.any(|item| item == constants::PATR_JWT_AUDIENCE)
	{
		warn!(
			"Invalid JWT audience: `{}`",
			match aud {
				OneOrMore::One(aud) => aud,
				OneOrMore::Multiple(aud) => format!("[{}]", aud.join(", ")),
			}
		);
		return Err(ErrorType::MalformedAccessToken);
	}

	let permissions =
		super::get_current_permissions_for_user(&mut *database, redis, &sid, &user.id.into())
			.await?;

	Ok(RequestUserData::builder()
		.id(user.id)
		.email(user.email)
		.first_name(user.first_name)
		.last_name(user.last_name)
		.created(user.created)
		.login_id(sid)
		.permissions(permissions)
		.build())
}
