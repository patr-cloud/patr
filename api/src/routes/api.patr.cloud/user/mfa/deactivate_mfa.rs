use axum::http::StatusCode;
use models::api::user::*;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::prelude::*;

pub async fn deactivate_mfa(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeactivateMfaPath,
				query: (),
				headers: DeactivateMfaRequestHeaders { authorization: _ },
				body: DeactivateMfaRequestProcessed { otp },
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
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

	AppResponse::builder()
		.body(DeactivateMfaResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
