use std::{
	collections::{BTreeMap, BTreeSet},
	net::IpAddr,
	ops::Sub,
};

use jsonwebtoken::{DecodingKey, TokenData, Validation};
use models::{
	IdentityData,
	RequestUserData,
	rbac::{ResourcePermissionType, WorkspacePermission},
	utils::ClientType,
};
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
		claims: AccessTokenData {
			iss,
			sub,
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
			"user".id,
			"user".username,
			"user".first_name,
			"user".last_name,
			"user".created
		FROM
			"user"
		INNER JOIN
			credential
		ON
			"user".id = credential.identity_id
		INNER JOIN
			web_login
		ON
			credential.credential_id = web_login.login_id
		WHERE
			credential.credential_id = $1 AND
			credential.type = 'web_login';
		"#,
		sub as _
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

	let permissions = super::get_permissions_for_identity(
		&mut *database,
		redis,
		&sub,
		&user.id.into(),
		ClientType::WebDashboard,
	)
	.await?;

	Ok(RequestUserData::builder()
		.id(user.id)
		.identity(IdentityData::User {
			username: user.username,
			first_name: user.first_name,
			last_name: user.last_name,
		})
		.client_type(ClientType::WebDashboard)
		.created(user.created)
		.login_id(sub)
		.permissions(permissions)
		.build())
}
