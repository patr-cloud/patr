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

use crate::{models::access_token_data::AccessTokenData, prelude::*};

pub async fn complete_sign_up(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: CompleteSignUpPath,
				query: (),
				headers: CompleteSignUpRequestHeaders { user_agent },
				body: CompleteSignUpRequestProcessed {
					username,
					verification_token,
				},
			},
		database,
		redis: _,
		client_ip,
		config,
	}: AppRequest<'_, CompleteSignUpRequest>,
) -> Result<AppResponse<CompleteSignUpRequest>, ErrorType> {
	info!("Completing sign up for user: `{username}`");

	let row = query!(
		r#"
        SELECT
            *
        FROM
            user_to_sign_up
        WHERE
            username = $1 AND
			otp_expiry > NOW();
        "#,
		&username
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)
	.inspect_err(|_| {
		info!("Could not find a row with the given username");
	})?;

	trace!("Found a row with the given username");

	let success = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
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
		SET CONSTRAINTS ALL DEFERRED;
		"#
	)
	.execute(&mut **database)
	.await?;

	trace!("Constraints deferred");

	query!(
		r#"
        INSERT INTO
            "user"(
                id,
                username,
                password,
                first_name,
                last_name,
                created,
                recovery_email,
                recovery_phone_country_code,
                recovery_phone_number,
                workspace_limit,
                password_reset_token,
                password_reset_token_expiry,
                password_reset_attempts,
                mfa_secret
            )
        VALUES
            (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                $10,
                NULL,
                NULL,
                NULL,
                NULL
            );
        "#,
		user_id as _,
		&username,
		row.password,
		row.first_name,
		row.last_name,
		now,
		row.recovery_email,
		row.recovery_phone_country_code,
		row.recovery_phone_number,
		constants::DEFAULT_WORKSPACE_LIMIT,
	)
	.execute(&mut **database)
	.await?;

	trace!("User inserted into the database");

	match (
		row.recovery_email,
		row.recovery_phone_country_code,
		row.recovery_phone_number,
	) {
		(Some(recovery_email), None, None) => {
			trace!("Inserting recovery email");
			query!(
				r#"
				INSERT INTO
					user_email(
						user_id,
						email
					)
				VALUES
					($1, $2);
				"#,
				user_id as _,
				recovery_email
			)
			.execute(&mut **database)
			.await?;
		}
		(None, Some(recovery_phone_country_code), Some(recovery_phone_number)) => {
			trace!("Inserting recovery phone number");
			query!(
				r#"
				INSERT INTO
					user_phone_number(
						user_id,
						country_code,
						number
					)
				VALUES
					($1, $2, $3);
				"#,
				user_id as _,
				recovery_phone_country_code,
				recovery_phone_number
			)
			.execute(&mut **database)
			.await?;
		}
		_ => {
			error!("No recovery email or phone number in user_to_sign_up table");
			return Err(ErrorType::server_error(
				"user_to_sign_up row has no recovery email or phone number",
			));
		}
	}

	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut **database)
	.await?;

	trace!("Constraints set to immediate");

	let refresh_token = Uuid::new_v4();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: {err}");
	})
	.map_err(ErrorType::server_error)?
	.hash_password(
		refresh_token.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.inspect_err(|err| {
		error!("Error hashing password: {err}");
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
	})
	.map_err(ErrorType::server_error)?
	.lookup(client_ip.to_string().as_str())
	.await
	.inspect_err(|err| {
		info!("Error looking up IP address: {err}");
	})
	.map_err(ErrorType::server_error)?;

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
		&EncodingKey::from_secret(config.jwt_secret.as_ref()),
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
