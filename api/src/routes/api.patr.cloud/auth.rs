use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
use axum::{http::StatusCode, Router};
use models::{
	// api::auth::{LoginRequest, LoginResponse},
	ApiRequest,
	ErrorType,
};

use crate::prelude::*;

#[instrument(skip(state))]
pub fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.with_state(state.clone())
		// .mount_endpoint(login, state)
}

// async fn login(
// 	AppRequest {
// 		request: ApiRequest {
// 			path: _,
// 			query: _,
// 			headers: _,
// 			body,
// 		},
// 		database,
// 		redis,
// 		client_ip,
// 		config,
// 	}: AppRequest<'_, LoginRequest>,
// ) -> Result<AppResponse<LoginRequest>, ErrorType> {
// 	let user_data = query!(
// 		r#"
// 		SELECT
// 			"user".username,
// 			"user".password
// 		FROM
// 			"user"
// 		LEFT JOIN
// 			personal_email
// 		ON
// 			personal_email.user_id = "user".id
// 		LEFT JOIN
// 			domain
// 		ON
// 			domain.id = personal_email.domain_id
// 		LEFT JOIN
// 			user_phone_number
// 		ON
// 			user_phone_number.user_id = "user".id
// 		LEFT JOIN
// 			phone_number_country_code
// 		ON
// 			phone_number_country_code.country_code = user_phone_number.country_code
// 		WHERE
// 			"user".username = $1 OR
// 			CONCAT(
// 				personal_email.local,
// 				'@',
// 				domain.name,
// 				'.',
// 				domain.tld
// 			) = $1 OR
// 			CONCAT(
// 				'+',
// 				phone_number_country_code.phone_code,
// 				user_phone_number.number
// 			) = $1;
// 		"#,
// 		""
// 	)
// 	.fetch_optional(&mut **database)
// 	.await?
// 	.ok_or(ErrorType::UserNotFound)?;

// 	let success = argon2::Argon2::new_with_secret(
// 		config.password_pepper.as_ref(),
// 		Algorithm::Argon2id,
// 		Version::V0x13,
// 		constants::HASHING_PARAMS,
// 	)
// 	.map_err(|err| ErrorType::server_error(err.to_string()))?
// 	.verify_password(
// 		&body.password,
// 		&PasswordHash::new(&user_data.password)?,
// 	)
// 	.is_ok();

// 	if !success {
// 		return Err(ErrorType::InvalidPassword);
// 	}

// 	AppResponse::builder()
// 		.body(LoginResponse {

// 		})
// 		.headers(())
// 		.status_code(StatusCode::ACCEPTED)
// 		.build()
// 		.into_result()
// }
