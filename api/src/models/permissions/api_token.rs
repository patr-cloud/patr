use std::net::IpAddr;

use argon2::{Algorithm, Argon2, PasswordHash, PasswordVerifier as _, Version};
use models::RequestUserData;
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

	info!("Extracting information about API token");
	let Some(token) = query!(
		r#"
		SELECT
			user_api_token.token_id,
			user_api_token.user_id,
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
	.fetch_optional(&mut *database) // What the actual fuck?
	.await?
	else {
		warn!("API token not found");
		// No specific error for API token not found, since we don't want to leak
		// information about whether a loginId is valid or if it's expired
		return Err(ErrorType::AuthorizationTokenInvalid);
	};
	trace!("Token extracted from database");

	if let Some(nbf) = token.token_nbf {
		trace!("Token has an NBF");
		if OffsetDateTime::now_utc() < nbf {
			info!("API token is not valid yet");
			return Err(ErrorType::AuthorizationTokenInvalid);
		}
	} else {
		trace!("Token does not have an NBF");
	}
	trace!("Token passed NBF check");

	if let Some(exp) = token.token_exp {
		trace!("Token has an EXP");
		if OffsetDateTime::now_utc() > exp {
			info!("API token has expired");
			return Err(ErrorType::AuthorizationTokenInvalid);
		}
	} else {
		trace!("Token does not have an EXP");
	}
	trace!("Token passed EXP check");

	if let Some(revoked) = token.revoked {
		trace!("Token has a revoked timestamp");
		if OffsetDateTime::now_utc() > revoked {
			info!("API token has been revoked");
			return Err(ErrorType::AuthorizationTokenInvalid);
		}
	} else {
		trace!("Token does not have a revoked timestamp");
	}
	trace!("Token passed revoked timestamp check");

	if let Some(allowed_ips) = token.allowed_ips &&
		!allowed_ips
			.iter()
			.any(|ip_network| ip_network.contains(client_ip))
	{
		info!("API token not accessed from an allowed IP Address");
		return Err(ErrorType::DisallowedIpAddressForApiToken);
	}

	let Ok(password_hash) = PasswordHash::new(&token.token_hash) else {
		error!("Unable to parse password hash: {}", token.token_hash);
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
		warn!("API token has invalid refresh token");
		return Err(ErrorType::AuthorizationTokenInvalid);
	}
	info!("API token valid");

	let permissions = super::get_permissions_for_login_id(
		&mut *database,
		redis,
		&login_id,
		&token.user_id.into(),
	)
	.await?;

	Ok(RequestUserData::builder()
		.id(token.user_id)
		.username(token.username)
		.first_name(token.first_name)
		.last_name(token.last_name)
		.created(token.created)
		.login_id(token.token_id)
		.permissions(permissions)
		.build())
}
