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
use models::api::auth::{
	CompleteSignUpPath,
	CompleteSignUpRequest,
	CompleteSignUpRequestHeaders,
	CompleteSignUpRequestProcessed,
	CompleteSignUpResponse,
};
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
		username
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	trace!("Found a row with the given username");

	let success = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.verify_password(
		verification_token.as_ref(),
		&PasswordHash::new(&row.otp_hash).map_err(ErrorType::server_error)?,
	)
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
		username,
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

	let login_id = Uuid::new_v4();

	let refresh_token = Uuid::new_v4();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.hash_password(
		refresh_token.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.map_err(ErrorType::server_error)?
	.to_string();
	let refresh_token_expiry = now.add(constants::INACTIVE_REFRESH_TOKEN_VALIDITY);

	let ip_info = ipinfo::IpInfo::new(ipinfo::IpInfoConfig {
		token: config.ipinfo.map(|ipinfo| ipinfo.token),
		..Default::default()
	})
	.map_err(ErrorType::server_error)?
	.lookup(client_ip.to_string().as_str())
	.await
	.map_err(ErrorType::server_error)?;

	if ip_info.bogon.unwrap_or(false) {
		return Err(ErrorType::server_error(format!(
			"cannot use bogon IP address: `{}`",
			client_ip
		)));
	}

	let client_ip = IpNetwork::from(client_ip);

	let (lat, lng) = ip_info
		.loc
		.split_once(',')
		.map(|(lat, lng)| Ok::<_, ParseFloatError>((lat.parse::<f64>()?, lng.parse::<f64>()?)))
		.ok_or_else(|| {
			ErrorType::server_error(format!("unknown latitude and longitude: {}", ip_info.loc))
		})?
		.map_err(ErrorType::server_error)?;
	let country = ip_info.country;
	let region = ip_info.region;
	let city = ip_info.city;
	let timezone = ip_info.timezone.unwrap_or_else(Default::default);

	let user_agent = user_agent.to_string();

	query!(
		r#"
        INSERT INTO
            user_login(
                login_id,
                user_id,
                login_type,
                created
            )
        VALUES
            ($1, $2, 'web_login', $3);
        "#,
		login_id as _,
		user_id as _,
		now
	)
	.execute(&mut **database)
	.await?;

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
	.map_err(ErrorType::server_error)?;

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
