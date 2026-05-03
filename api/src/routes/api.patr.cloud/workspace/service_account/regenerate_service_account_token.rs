use argon2::{Algorithm, Argon2, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::workspace::service_account::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn regenerate_service_account_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					RegenerateServiceAccountTokenPath {
						workspace_id: _,
						service_account_id,
					},
				query: (),
				headers:
					RegenerateServiceAccountTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RegenerateServiceAccountTokenRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, RegenerateServiceAccountTokenRequest>,
) -> Result<AppResponse<RegenerateServiceAccountTokenRequest>, ErrorType> {
	let refresh_token = Uuid::new_v4();
	let token_hash = Argon2::new_with_secret(
		state.config.password_pepper.as_bytes(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token.as_bytes(), &generate_salt())
	.map_err(ErrorType::server_error)?
	.to_string();

	let rows_affected = query!(
		r#"
		UPDATE
			service_account
		SET
			token_hash = $1
		WHERE
			id = $2 AND
			deleted IS NULL;
		"#,
		&token_hash,
		service_account_id as _,
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	if rows_affected == 0 {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	// Invalidate cached permissions for the old token
	redis
		.setex(
			redis::keys::user_id_revocation_timestamp(&service_account_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await?;

	let token = format!("patrv1.{}.{}", refresh_token, service_account_id);

	AppResponse::builder()
		.body(RegenerateServiceAccountTokenResponse { token })
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
