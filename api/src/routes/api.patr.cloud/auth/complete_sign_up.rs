use models::api::auth::*;

use crate::prelude::*;

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
		state,
	}: AppRequest<'_, CompleteSignUpRequest>,
) -> Result<AppResponse<CompleteSignUpRequest>, ErrorType> {
	cfg_if! {
		if #[cfg(not(feature = "cloud"))] {
			// Self-hosted instances don't allow self-service sign-up — users
			// are seeded / invited by the operator. Mirror the frontend, which
			// 404s the sign-up routes.
			let _ = (
				email,
				verification_token,
				cf_turnstile_token,
				user_agent,
				database,
				client_ip,
				state,
			);
			Err(ErrorType::FeatureNotSupported)
		} else {
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
			use sqlx::types::ipnetwork::IpNetwork;
			use time::OffsetDateTime;

			use crate::models::access_token_data::AccessTokenData;
			use crate::utils::cloudflare::validate_turnstile_token;

			trace!("Validating Cloudflare Turnstile token");
			let cf_turnstile_response = validate_turnstile_token(
				&state.config.cloudflare.turnstile_secret,
				&cf_turnstile_token,
				client_ip,
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
					email = $1::CITEXT AND
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

			// Mirror reset_password's attempt counter — gate at the same MAX so a
			// brute-force attempt on the 6-digit OTP exhausts cheaply and
			// cumulatively (no per-cycle reset). The increment fires for every
			// check, success or not; on success the row is DELETEd below so the
			// value never matters again.
			if row.sign_up_attempts >= constants::MAX_SIGN_UP_ATTEMPTS {
				debug!("Sign up attempts exceeded");
				return Err(ErrorType::UserNotFound);
			}

			// Counted on the pool, not the request transaction: a wrong OTP
			// returns an `Err`, which rolls the transaction back. An increment
			// written there would be discarded along with it, leaving the
			// ceiling above permanently unreachable.
			query!(
				r#"
				UPDATE
					user_to_sign_up
				SET
					sign_up_attempts = sign_up_attempts + 1
				WHERE
					email = $1::CITEXT;
				"#,
				&email,
			)
			.execute(&state.database)
			.await?;

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
				row.password,
				row.first_name,
				row.last_name,
				now,
				// `row.email`, not the request's — they match case-insensitively
				// but may differ in casing, and the address they signed up with
				// is the one worth keeping.
				row.email,
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
					email = $1::CITEXT;
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

			let ip_info = ip::lookup(client_ip, &state).await?;

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
				WITH client AS (
					INSERT INTO
						actor_client(id, actor_client_type)
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
				RETURNING user_login.login_id;
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
	}
}
