use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::{SetExpiration, StringCommands as _};
use time::OffsetDateTime;
use totp_rs::{Algorithm as TotpAlgorithm, Builder, Secret};

use crate::{prelude::*, redis::keys as redis};

pub async fn deactivate_mfa(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeactivateMfaPath,
				query: (),
				headers:
					DeactivateMfaRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeactivateMfaRequestProcessed { otp },
			},
		database,
		redis,
		client_ip: _,
		state: _,
		user_data,
	}: AuthenticatedAppRequest<'_, DeactivateMfaRequest>,
) -> Result<AppResponse<DeactivateMfaRequest>, ErrorType> {
	info!("Deactivating MFA for user");

	let mfa_detail = query!(
		r#"
		SELECT
			"user".mfa_secret
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_data.id
	)
	.fetch_one(&mut **database)
	.await?;

	let Some(secret) = mfa_detail.mfa_secret else {
		return Err(ErrorType::MfaAlreadyInactive);
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
			mfa_secret = NULL
		WHERE
			id = $1;
		"#,
		user_data.id
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
		.body(DeactivateMfaResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
