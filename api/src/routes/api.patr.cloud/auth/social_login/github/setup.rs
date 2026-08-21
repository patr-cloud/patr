use std::{num::ParseFloatError, ops::Add};

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use jsonwebtoken::EncodingKey;
use models::api::auth::*;
use rustis::commands::StringCommands;
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;

use crate::{
	models::{access_token_data::AccessTokenData, social_login::GithubSetupPayload},
	prelude::*,
};

/// `POST /auth/social-login/{provider}/setup`
///
/// Creates a new Patr account from a confirmed social-login identity after
/// the user has reviewed/edited the pre-filled profile details.
pub async fn social_login_setup(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: SocialLoginSetupPath { provider },
				query: (),
				headers: SocialLoginSetupRequestHeaders { user_agent },
				body:
					SocialLoginSetupRequestProcessed {
						setup_token,
						first_name,
						last_name,
					},
			},
		database,
		redis,
		client_ip,
		state,
	}: AppRequest<'_, SocialLoginSetupRequest>,
) -> Result<AppResponse<SocialLoginSetupRequest>, ErrorType> {
	trace!("Processing {provider} OAuth account setup");

	#[expect(irrefutable_let_patterns)]
	let SocialLoginProvider::GitHub = provider else {
		return Err(ErrorType::SocialLoginFailed);
	};

	// Atomically fetch-and-consume the setup token. `GETDEL` ensures two
	// concurrent requests with the same token cannot both observe it as valid.
	let setup_key = redis::keys::social_login_setup(&provider, &setup_token);

	let payload = serde_json::from_str::<GithubSetupPayload>(
		&redis
			.getdel::<Option<String>>(&setup_key)
			.await
			.inspect_err(|err| error!("Redis error consuming setup token: {err}"))?
			.ok_or(ErrorType::SocialLoginFailed)?,
	)?;

	// Check email availability
	let email_taken = query!(
		r#"
		SELECT
			id
		FROM
			"user"
		WHERE
			email = $1::CITEXT;
		"#,
		&payload.email,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if email_taken {
		return Err(ErrorType::EmailUnavailable);
	}

	let now = OffsetDateTime::now_utc();
	let user_id = Uuid::new_v4();

	// Hash a random password (user authenticates via GitHub; can reset via email)
	let random_password = Uuid::new_v4().to_string();
	let hashed_password = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| error!("Error creating Argon2: {err}"))
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(random_password.as_bytes(), &generate_salt())
	.inspect_err(|err| error!("Error hashing password: {err}"))
	.map_err(ErrorType::server_error)?
	.to_string();

	query!(
		r#"
		INSERT INTO
			"user"(
				id,
				password,
				first_name,
				last_name,
				created,
				email,
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
				NULL,
				NULL,
				NULL,
				NULL
			);
		"#,
		user_id as _,
		hashed_password,
		&first_name,
		&last_name,
		now,
		&payload.email,
		constants::DEFAULT_WORKSPACE_LIMIT,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_social_login(
				user_id,
				provider,
				external_id,
				linked_at
			)
		VALUES
			($1, 'github', $2, $3);
		"#,
		user_id as _,
		payload.external_id,
		now,
	)
	.execute(&mut **database)
	.await?;

	let refresh_token = Uuid::new_v4().to_string();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| error!("Error creating Argon2: {err}"))
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token.as_bytes(), &generate_salt())
	.inspect_err(|err| error!("Error hashing refresh token: {err}"))
	.map_err(ErrorType::server_error)?
	.to_string();
	let refresh_token_expiry = now.add(constants::INACTIVE_REFRESH_TOKEN_VALIDITY);

	let ip_info = ip::lookup(client_ip, &state).await?;

	if !cfg!(debug_assertions) && ip_info.bogon.unwrap_or(false) {
		return Err(ErrorType::server_error(format!(
			"cannot use bogon IP address: `{}`",
			client_ip
		)));
	}

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
	let timezone = ip_info.timezone.unwrap_or_default();

	let user_agent = user_agent.to_string();

	let login_id = query!(
		r#"
		WITH client AS (
			INSERT INTO
				actor_client(id, client_type)
			VALUES
				(GENERATE_LOGIN_ID(), 'user_login')
			RETURNING id
		)
		INSERT INTO
			user_login(
				login_id,
				user_id,
				login_type,
				created
			)
		SELECT
			client.id,
			$1,
			'web_login',
			$2
		FROM
			client
		RETURNING user_login.login_id AS "login_id: Uuid";
		"#,
		user_id as _,
		now,
	)
	.fetch_one(&mut **database)
	.await?
	.login_id;

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
		IpNetwork::from(client_ip),
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
		&EncodingKey::from_secret(state.config.jwt_secret.as_ref()),
	)
	.inspect_err(|err| error!("Error encoding JWT: {err}"))?;

	let refresh_token = format!("{login_id}.{refresh_token}");

	AppResponse::builder()
		.body(SocialLoginSetupResponse {
			access_token,
			refresh_token,
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
