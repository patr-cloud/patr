use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::StringCommands as _;
use time::OffsetDateTime;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

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
		user_data.id as _
	)
	.fetch_one(&mut **database)
	.await?;

	let Some(secret) = mfa_detail.mfa_secret else {
		return Err(ErrorType::MfaAlreadyInactive);
	};

	let mfa_valid = TOTP::new(
		TotpAlgorithm::SHA1,
		6,
		1,
		30,
		Secret::Encoded(secret.clone())
			.to_bytes()
			.inspect_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
			})?,
		Some(constants::TOTP_ISSUER.to_string()),
		user_data
			.identity
			.username()
			.ok_or(ErrorType::Unauthorized)?
			.to_string(),
	)
	.inspect_err(|err| {
		error!(
			"Unable to parse TOTP for userId `{}`: {}",
			user_data.id,
			err.to_string()
		);
	})?
	.check_current(&otp)?;

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
		user_data.id as _
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
		user_data.id as _,
		user_data.login_id as _,
	)
	.execute(&mut **database)
	.await?;

	redis
		.setex(
			redis::user_id_revocation_timestamp(&user_data.id.into()),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
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
