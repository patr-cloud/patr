use argon2::{password_hash::SaltString, Algorithm, PasswordHasher, Version};
use http::StatusCode;
use models::api::auth::*;

use crate::prelude::*;

/// The `sign_up` endpoint is used to create a new account.
pub async fn sign_up(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: CreateAccountPath,
				query: (),
				headers: CreateAccountRequestHeaders { user_agent: _ },
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
		runner_changes_sender: _,
		config,
	}: AppRequest<'_, CreateAccountRequest>,
) -> Result<AppResponse<CreateAccountRequest>, ErrorType> {
	trace!("Signing up user: {}", username);

	let rows = query(
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

	let mut db_user_id = None;
	let mut db_password_hash = None;

	for row in rows {
		let id = row.try_get::<String, _>("id")?;
		let value = row.try_get::<String, _>("value")?;

		match id.as_str() {
			constants::USER_ID_KEY => {
				db_user_id = Some(value);
			}
			constants::PASSWORD_HASH_KEY => {
				db_password_hash = Some(value);
			}
			_ => (),
		}
	}

	let None = db_user_id.zip(db_password_hash) else {
		return Err(ErrorType::UsernameUnavailable);
	};

	trace!("Creating user with username: {}", username);

	let RunnerMode::SelfHosted {
		password_pepper,
		jwt_secret: _, // Not needed for sign up
	} = config.mode
	else {
		return Err(ErrorType::InvalidRunnerMode);
	};

	let hashed_password = argon2::Argon2::new_with_secret(
		password_pepper.as_ref(),
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
