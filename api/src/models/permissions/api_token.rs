use std::net::IpAddr;

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier as _, Version};
use models::{IdentityData, RequestUserData, utils::ClientType};
use rustis::client::Client as RedisClient;
use time::OffsetDateTime;

use crate::{prelude::*, utils::config::AppConfig};

pub(crate) async fn get_permissions(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	config: &AppConfig,
	client_ip: IpAddr,
	token: &str,
) -> Result<RequestUserData, ErrorType> {
	trace!("Parsing authentication header as an API token");
	let (refresh_token, login_id) = token
		.strip_prefix("patrv1.")
		.ok_or_else(|| {
			warn!("Invalid API token provided: {}", token);
			ErrorType::MalformedApiToken
		})?
		.split_once('.')
		.ok_or_else(|| {
			warn!("Invalid API token provided: {}", token);
			ErrorType::MalformedApiToken
		})?;

	let refresh_token = Uuid::parse_str(refresh_token).map_err(|err| {
		warn!("Invalid API token provided: {}", token);
		warn!(
			"Cannot parse refresh token `{}` as UUID: {}",
			refresh_token, err
		);
		ErrorType::MalformedApiToken
	})?;
	trace!("Refresh token parsed as UUID");

	let login_id = Uuid::parse_str(login_id).map_err(|err| {
		warn!("Invalid API token provided: {}", token);
		warn!("Cannot parse loginId `{}` as UUID: {}", login_id, err);
		ErrorType::MalformedApiToken
	})?;
	trace!("Login ID parsed as UUID");

	// Resolve the token to an identity. The branches extract:
	// (identity_id, login_id, identity_created_at, token_hash, identity_data,
	// client_type)
	//
	// We try user_api_token first, then fall back to service_account. A UUIDv4
	// collision between user_login.login_id and service_account.id is
	// vanishingly unlikely, but even if it happened the worst case is the SA
	// can't authenticate (the user_api_token branch matches first, then the
	// hash check fails because the hashes don't match). No unauthorized access
	// is possible — just a soft-bricked SA.
	info!("Extracting information about API token");
	let (
		identity_id,
		resolved_login_id,
		identity_created_at,
		token_hash,
		identity,
		resolved_client_type,
	) = if let Some(token) = query!(
		r#"
		SELECT
			user_api_token.token_id AS "token_id: Uuid",
			user_api_token.user_id AS "user_id: Uuid",
			user_api_token.token_hash,
			user_api_token.token_nbf,
			user_api_token.token_exp,
			user_api_token.allowed_ips,
			user_api_token.revoked,
			"user".*
		FROM
			user_api_token
		INNER JOIN
			user_login
		ON
			user_api_token.token_id = user_login.login_id
		INNER JOIN
			"user"
		ON
			user_api_token.user_id = "user".id
		WHERE
			user_api_token.token_id = $1 AND
			user_login.login_type = 'api_token';
		"#,
		login_id as _
	)
	.fetch_optional(&mut *database)
	.await?
	{
		trace!("Found user API token");

		if let Some(nbf) = token.token_nbf {
			if OffsetDateTime::now_utc() < nbf {
				info!("API token is not valid yet");
				return Err(ErrorType::AuthorizationTokenInvalid);
			}
		}

		if let Some(exp) = token.token_exp {
			if OffsetDateTime::now_utc() > exp {
				info!("API token has expired");
				return Err(ErrorType::AuthorizationTokenInvalid);
			}
		}

		if let Some(revoked) = token.revoked {
			if OffsetDateTime::now_utc() > revoked {
				info!("API token has been revoked");
				return Err(ErrorType::AuthorizationTokenInvalid);
			}
		}

		if let Some(allowed_ips) = token.allowed_ips &&
			!allowed_ips
				.iter()
				.any(|ip_network| ip_network.contains(client_ip))
		{
			info!("API token not accessed from an allowed IP Address");
			return Err(ErrorType::DisallowedIpAddressForApiToken);
		}

		(
			token.user_id,
			token.token_id,
			token.created,
			token.token_hash,
			IdentityData::User {
				username: token.username,
				first_name: token.first_name,
				last_name: token.last_name,
			},
			ClientType::ApiToken,
		)
	} else if let Some(service_account) = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name,
			token_hash,
			created
		FROM
			service_account
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		login_id as _
	)
	.fetch_optional(&mut *database)
	.await?
	{
		trace!("Found service account token");

		(
			service_account.id,
			service_account.id,
			service_account.created,
			service_account.token_hash,
			IdentityData::ServiceAccount {
				name: service_account.name,
			},
			ClientType::ServiceAccount,
		)
	} else {
		warn!("Token not found as user API token or service account");
		return Err(ErrorType::AuthorizationTokenInvalid);
	};

	// Verify the token hash
	let Ok(password_hash) = PasswordHash::new(&token_hash) else {
		error!("Unable to parse password hash: {}", token_hash);
		return Err(ErrorType::server_error("password hash parsing failed"));
	};
	let success = Argon2::new_with_secret(
		config.password_pepper.as_bytes(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.verify_password(refresh_token.as_bytes(), &password_hash)
	.is_ok();

	if !success {
		warn!("Token has invalid refresh token");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	info!("Token valid");

	let permissions = super::get_permissions_for_login_id(
		&mut *database,
		redis,
		&resolved_login_id,
		&identity_id,
	)
	.await?;

	Ok(RequestUserData::builder()
		.id(identity_id)
		.identity(identity)
		.client_type(resolved_client_type)
		.created(identity_created_at)
		.login_id(resolved_login_id)
		.permissions(permissions)
		.build())
}
