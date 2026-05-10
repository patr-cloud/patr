use std::{num::ParseFloatError, ops::Add};

use argon2::{
	Algorithm,
	PasswordHash,
	PasswordHasher,
	PasswordVerifier,
	Version,
	password_hash::generate_salt,
};
use axum::http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;

use crate::{
	models::access_token_data::AccessTokenData,
	prelude::*,
	utils::cloudflare::validate_turnstile_token,
};

pub async fn complete_sign_up(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: CompleteSignUpPath,
				query: (),
				headers: CompleteSignUpRequestHeaders { user_agent },
				body:
					CompleteSignUpRequestProcessed {
						email,
						verification_token,
						cf_turnstile_token,
					},
			},
		database,
		redis: _,
		client_ip,
		mut state,
	}: AppRequest<'_, CompleteSignUpRequest>,
) -> Result<AppResponse<CompleteSignUpRequest>, ErrorType> {
	trace!("Validating Cloudflare Turnstile token");
	let cf_turnstile_response = validate_turnstile_token(
		&state.config.cloudflare.turnstile_secret,
		&cf_turnstile_token,
		Some(client_ip),
	)
	.await
	.inspect_err(|err| {
		error!("Error verifying Cloudflare Turnstile token: `{}`", err);
	})?;

	if !cf_turnstile_response.success {
		return Err(ErrorType::TurnstileVerificationFailed);
	}

	if !cfg!(debug_assertions) && &cf_turnstile_response.action != "complete-sign-up" {
		return Err(ErrorType::TurnstileVerificationActionMismatch);
	}

	info!("Completing sign up for user: `{email}`");

	let row = query!(
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			email = $1 AND
			otp_expiry > NOW();
		"#,
		&email
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)
	.inspect_err(|_| {
		info!("Could not find a row with the given email");
	})?;

	trace!("Found a row with the given email");

	let success = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: {err}");
	})
	.map_err(ErrorType::server_error)?
	.verify_password(
		verification_token.as_bytes(),
		&PasswordHash::new(&row.otp_hash).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		info!("Error verifying password: {err}");
	})
	.is_ok();

	if !success {
		debug!("Verification token hash is invalid");
		return Err(ErrorType::UserNotFound);
	}

	trace!("Verification token hash is validated");

	// User is valid. Now create a login and send back the credentials

	let now = OffsetDateTime::now_utc();
	let user_id = Uuid::new_v4();

	query!(
		r#"
		INSERT INTO
			"user"(
				id,
				email,
				password,
				first_name,
				last_name,
				created,
				workspace_limit,
				password_reset_token,
				password_reset_token_expiry,
				password_reset_attempts,
				mfa_secret
			)
		VALUES
			(
				$1, $2, $3, $4, $5, $6, $7, NULL, NULL, NULL, NULL
			);
		"#,
		user_id as _,
		&email,
		row.password,
		row.first_name,
		row.last_name,
		now,
		constants::DEFAULT_WORKSPACE_LIMIT,
	)
	.execute(&mut **database)
	.await?;

	trace!("User inserted into the database");

	query!(
		r#"
		DELETE FROM
			user_to_sign_up
		WHERE
			email = $1;
		"#,
		&email
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted user_to_sign_up entry");

	let refresh_token = Uuid::new_v4().to_string();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: {err}");
	})
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token.as_ref(), &generate_salt())
	.inspect_err(|err| {
		error!("Error hashing password: {err}");
	})
	.map_err(ErrorType::server_error)?
	.to_string();
	let refresh_token_expiry = now.add(constants::INACTIVE_REFRESH_TOKEN_VALIDITY);

	let ip_info = ip::lookup(client_ip, &mut state.redis, &state.config.ipinfo).await?;

	if !cfg!(debug_assertions) && ip_info.bogon.unwrap_or(false) {
		return Err(ErrorType::server_error(format!(
			"cannot use bogon IP address: `{}`",
			client_ip
		)));
	}

	let client_ip = IpNetwork::from(client_ip);

	let (lat, lng) = if cfg!(debug_assertions) {
		(0f64, 0f64)
	} else {
		ip_info
			.loc
			.split_once(',')
			.map(|(lat, lng)| {
				Ok::<_, ParseFloatError>((
					lat.parse::<f64>().inspect_err(|err| {
						info!("Error parsing latitude: `{lat}` - {err}");
					})?,
					lng.parse::<f64>().inspect_err(|err| {
						info!("Error parsing longitude: `{lng}` - {err}");
					})?,
				))
			})
			.ok_or_else(|| {
				ErrorType::server_error(format!("unknown latitude and longitude: {}", ip_info.loc))
			})??
	};
	let country = ip_info.country;
	let region = ip_info.region;
	let city = ip_info.city;
	let timezone = ip_info.timezone.unwrap_or_else(Default::default);

	let user_agent = user_agent.to_string();

	let login_id = query!(
		r#"
		INSERT INTO
			user_login(
				login_id,
				user_id,
				login_type,
				created
			)
		VALUES
			(
				GENERATE_LOGIN_ID(),
				$1,
				'web_login',
				$2
			)
		RETURNING login_id;
		"#,
		user_id as _,
		now,
	)
	.fetch_one(&mut **database)
	.await?
	.login_id
	.into();

	trace!("User login inserted into the database");

	query!(
		r#"
		INSERT INTO
			web_login(
				login_id,
				original_login_id,
				user_id,
	
				refresh_token,
				token_expiry,
	
				created,
				created_ip,
				created_location,
				created_user_agent,
				created_country,
				created_region,
				created_city,
				created_timezone
			)
		VALUES
			(
				$1,
				NULL,
				$2,

				$3,
				$4,

				$5,
				$6,
				ST_SetSRID(POINT($7, $8)::GEOMETRY, 4326),
				$9,
				$10,
				$11,
				$12,
				$13
			);
		"#,
		login_id as _,
		user_id as _,
		hashed_refresh_token,
		refresh_token_expiry,
		now,
		client_ip,
		lat,
		lng,
		user_agent,
		country,
		region,
		city,
		timezone,
	)
	.execute(&mut **database)
	.await?;

	trace!("Web login inserted into the database");

	let access_token = AccessTokenData {
		iss: constants::JWT_ISSUER.to_string(),
		sub: login_id,
		aud: OneOrMore::One(constants::PATR_JWT_AUDIENCE.to_string()),
		exp: now.add(constants::ACCESS_TOKEN_VALIDITY),
		nbf: now,
		iat: now,
		jti: Uuid::now_v1(),
	};
	let access_token = jsonwebtoken::encode(
		&Default::default(),
		&access_token,
		&EncodingKey::from_secret(state.config.jwt_secret.as_ref()),
	)
	.inspect_err(|err| {
		info!("Error encoding JWT: {err}");
	})?;

	let refresh_token = format!("{login_id}.{refresh_token}");

	AppResponse::builder()
		.body(CompleteSignUpResponse {
			access_token,
			refresh_token,
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
