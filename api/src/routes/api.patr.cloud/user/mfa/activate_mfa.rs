use axum::http::StatusCode;
use models::{api::user::*, RequestUserData};
use rustis::commands::StringCommands;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::{prelude::*, redis::keys as redis};

pub async fn activate_mfa(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ActivateMfaPath,
				query: (),
				headers: ActivateMfaRequestHeaders { authorization: _ },
				body: ActivateMfaRequestProcessed { otp },
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: RequestUserData { id, .. },
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
		id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::UserNotFound)?;

	if mfa_detail.mfa_secret.is_some() {
		return Err(ErrorType::MfaAlreadyActive);
	}

	let Some(secret) = redis
		.get::<_, Option<String>>(redis::user_mfa_secret(&id))
		.await?
	else {
		error!("MFA secret not found for userId `{}`", id);
		return Err(ErrorType::MfaRequired);
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
					id,
					err.to_string()
				);
			})?,
	)
	.inspect_err(|err| {
		error!(
			"Unable to parse TOTP for userId `{}`: {}",
			id,
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
			mfa_secret = $2
		WHERE
			id = $1;
		"#,
		id as _,
		secret
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(ActivateMfaResponse)
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
