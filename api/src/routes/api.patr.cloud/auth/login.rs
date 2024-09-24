use std::{num::ParseFloatError, ops::Add};

use argon2::{
	password_hash::SaltString,
	Algorithm,
	PasswordHash,
	PasswordHasher,
	PasswordVerifier,
	Version,
};
use axum::http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::{models::access_token_data::AccessTokenData, prelude::*};

/// The handler to login the user. This will return the access token and the
/// refresh token.
pub async fn login(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: LoginPath,
				query: (),
				headers: LoginRequestHeaders { user_agent },
				body: LoginRequestProcessed {
					user_id,
					password,
					mfa_otp,
				},
			},
		database,
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, LoginRequest>,
) -> Result<AppResponse<LoginRequest>, ErrorType> {
	trace!("Logging in user: {}", user_id);

	let user_data = query!(
		r#"
		SELECT
			"user".id,
			"user".username,
			"user".password,
			"user".mfa_secret
		FROM
			"user"
		LEFT JOIN
			user_email
		ON
			user_email.user_id = "user".id
		LEFT JOIN
			user_phone_number
		ON
			user_phone_number.user_id = "user".id
		LEFT JOIN
			phone_number_country_code
		ON
			phone_number_country_code.country_code = user_phone_number.country_code
		WHERE
			"user".username = $1 OR
			user_email.email = $1 OR
			CONCAT(
				'+',
				phone_number_country_code.phone_code,
				user_phone_number.number
			) = $1;
		"#,
		&user_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	trace!("Found user with ID: {}", user_data.id);

	let success = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.verify_password(
		password.as_bytes(),
		&PasswordHash::new(&user_data.password).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		info!("Error verifying password: `{}`", err);
	})
	.is_ok();

	if !success {
		return Err(ErrorType::InvalidPassword);
	}

	trace!("Password hashes match");

	if let Some(mfa_secret) = user_data.mfa_secret {
		trace!("User has MFA secret");

		let Some(mfa_otp) = mfa_otp else {
			return Err(ErrorType::MfaRequired);
		};

		let totp = TOTP::new(
			TotpAlgorithm::SHA1,
			6,
			1,
			30,
			Secret::Encoded(mfa_secret).to_bytes().inspect_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
			})?,
		)
		.inspect_err(|err| {
			error!(
				"Unable to parse TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})?;

		let mfa_valid = totp.check_current(&mfa_otp).inspect_err(|err| {
			error!(
				"System time error while checking TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})?;

		if !mfa_valid {
			return Err(ErrorType::MfaOtpInvalid);
		}

		trace!("User MFA is valid");
	}

	let now = OffsetDateTime::now_utc();

	let refresh_token = Uuid::new_v4();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.hash_password(
		refresh_token.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.inspect_err(|err| {
		error!("Error hashing refresh token: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();
	let refresh_token_expiry = now.add(constants::INACTIVE_REFRESH_TOKEN_VALIDITY);

	let ip_info = ipinfo::IpInfo::new(ipinfo::IpInfoConfig {
		token: { Some(config.ipinfo.token) },
		..Default::default()
	})
	.inspect_err(|err| {
		info!("Error creating IpInfo: {err}");
	})?
	.lookup(client_ip.to_string().as_str())
	.await
	.inspect_err(|err| {
		info!("Error looking up IP address: {err}");
	})?;

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
		user_data.id,
		now,
	)
	.fetch_one(&mut **database)
	.await?
	.login_id
	.into();

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
		user_data.id,
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
		&EncodingKey::from_secret(config.jwt_secret.as_ref()),
	)
	.inspect_err(|err| {
		error!("Error encoding JWT: `{}`", err);
	})?;

	let refresh_token = format!("{login_id}.{refresh_token}");

	AppResponse::builder()
		.body(LoginResponse {
			access_token,
			refresh_token,
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
