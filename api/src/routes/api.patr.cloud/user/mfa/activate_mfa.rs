use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::{SetExpiration, StringCommands};
use time::OffsetDateTime;
use totp_rs::{Algorithm as TotpAlgorithm, Builder, Secret};

use crate::{prelude::*, redis::keys as redis};

pub async fn activate_mfa(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ActivateMfaPath,
				query: (),
				headers:
					ActivateMfaRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ActivateMfaRequestProcessed { otp },
			},
		database,
		redis,
		client_ip: _,
		state: _,
		user_data,
	}: AuthenticatedAppRequest<'_, ActivateMfaRequest>,
) -> Result<AppResponse<ActivateMfaRequest>, ErrorType> {
	info!("Activating MFA for user");

	let mfa_detail = query!(
		r#"
		SELECT
			"user".mfa_secret
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_data.id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	if mfa_detail.mfa_secret.is_some() {
		return Err(ErrorType::MfaAlreadyActive);
	}

	let Some(secret) = redis
		.get::<Option<String>>(redis::user_mfa_secret(&user_data.id))
		.await?
	else {
		error!("MFA secret not found for userId `{}`", user_data.id);
		return Err(ErrorType::MfaRequired);
	};

	let mfa_valid = Builder::new()
		.with_algorithm(TotpAlgorithm::SHA1)
		.with_digits(6)
		.with_skew(1)
		.with_step_duration(30)
		.with_secret(Secret::try_from_base32(secret.clone()).inspect_err(|err| {
			error!(
				"Unable to parse MFA secret for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})?)
		.with_issuer(Some(constants::TOTP_ISSUER))
		.with_account_name(
			user_data
				.identity
				.email()
				.ok_or(ErrorType::Unauthorized)?
				.to_string(),
		)
		.build()
		.inspect_err(|err| {
			error!(
				"Unable to parse TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})?
		.check_current(&otp)
		.is_some();

	if !mfa_valid {
		return Err(ErrorType::MfaOtpInvalid);
	}

	query!(
		r#"
		UPDATE
			"user"
		SET
			mfa_secret = $2
		WHERE
			id = $1;
		"#,
		user_data.id,
		secret
	)
	.execute(&mut **database)
	.await?;

	// Drop every other web login the user has so a hijacked session can't
	// stick around past the MFA toggle. Keep the caller's session.
	query!(
		r#"
		DELETE FROM
			web_login
		WHERE
			user_id = $1 AND
			login_id != $2;
		"#,
		user_data.id,
		user_data.login_id,
	)
	.execute(&mut **database)
	.await?;

	redis
		.set_with_options(
			redis::user_id_revocation_timestamp(&user_data.id),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
			None,
			SetExpiration::Ex(
				constants::CACHED_PERMISSIONS_VALIDITY
					.whole_seconds()
					.unsigned_abs(),
			),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting user_id_revocation_timestamp: `{}`", err);
		})?;

	AppResponse::builder()
		.body(ActivateMfaResponse)
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
