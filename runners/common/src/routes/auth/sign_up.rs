use argon2::{password_hash::SaltString, Algorithm, PasswordHasher, Version};
use http::StatusCode;
use models::api::auth::*;

use crate::prelude::*;

/// The `sign_up` endpoint is used to create a new account.
pub async fn sign_up(
	request: AppRequest<'_, CreateAccountRequest>,
) -> Result<AppResponse<CreateAccountRequest>, ErrorType> {
	let AppRequest {
		config,
		request:
			ProcessedApiRequest {
				path: _,
				query: _,
				headers: _,
				body:
					CreateAccountRequestProcessed {
						first_name,
						last_name,
						username,
						password,
						recovery_method: _,
					},
			},
		database,
	} = request;

	let raw_user_data = query(
		r#"
		SELECT
			*
		FROM
			meta_data
		WHERE
			id = $1 OR
			id = $2;
		"#,
	)
	.bind(constants::USER_ID_KEY)
	.bind(constants::PASSWORD_HASH_KEY)
	.fetch_all(&mut **database)
	.await?;

	let mut user_data = UserData::new();

	for row in raw_user_data {
		let id = row.try_get::<String, &str>("id")?;
		let value = row.try_get::<String, &str>("value")?;

		match id.as_str() {
			"user_id" => {
				user_data.user_id = value;
			}
			"password_hash" => {
				user_data.password_hash = value;
			}
			_ => {}
		}
	}

	if user_data.is_user_available() {
		return Err(ErrorType::server_error("User Already Exists"));
	}
	trace!("Creating user with username: {}", username);

	let hashed_password = argon2::Argon2::new_with_secret(
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.hash_password(
		password.as_bytes(),
		SaltString::generate(&mut rand::thread_rng()).as_salt(),
	)
	.inspect_err(|err| {
		error!("Error hashing password: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	query(
		r#"
		INSERT INTO
			meta_data(
				id,
				value
			)
		VALUES
			($1, $2),
			($3, $4),
			($5, $6),
			($7, $8);
		"#,
	)
	.bind(constants::USER_ID_KEY)
	.bind(&username)
	.bind(constants::PASSWORD_HASH_KEY)
	.bind(hashed_password)
	.bind(constants::FIRST_NAME_KEY)
	.bind(first_name)
	.bind(constants::LAST_NAME_KEY)
	.bind(last_name)
	.execute(&mut **database)
	.await?;

	trace!("User inserted into the database");

	AppResponse::builder()
		.body(CreateAccountResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
