use eve_rs::AsError;

use crate::{
	db,
	error,
	service,
	utils::{get_current_time_millis, validator, Error},
	Database,
};

pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	email_address: &str,
	user_id: &[u8],
) -> Result<(), Error> {
	if !validator::is_email_valid(email_address) {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_EMAIL).to_string())?;
	}

	if db::get_user_by_email(connection, &email_address)
		.await?
		.is_some()
	{
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_TAKEN).to_string())?;
	}

	let otp = service::generate_new_otp();

	let token_expiry =
		get_current_time_millis() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	let (email_local, domain_name) = email_address
		.split_once('@')
		.status(400)
		.body(error!(INVALID_EMAIL).to_string())?;

	let personal_domain_id =
		service::ensure_personal_domain_exists(connection, domain_name).await?;

	db::add_personal_email_to_be_verified_for_user(
		connection,
		&email_local,
		personal_domain_id.as_bytes(),
		&user_id,
		&verification_token,
		token_expiry,
	)
	.await?;

	Ok(())
}

pub async fn verify_personal_email_address_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	email_address: &str,
	otp: &str,
) -> Result<(), Error> {
	let email_verification_data =
		db::get_personal_email_to_be_verified_for_user(
			connection,
			user_id,
			email_address,
		)
		.await?
		.status(400)
		.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	let success = service::validate_hash(
		otp,
		&email_verification_data.verification_token_hash,
	)?;
	if !success {
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;
	}

	if email_verification_data.verification_token_expiry <
		get_current_time_millis()
	{
		Error::as_result()
			.status(200)
			.body(error!(EMAIL_TOKEN_EXPIRED).to_string())?;
	}

	db::add_personal_email_for_user(
		connection,
		user_id,
		&email_verification_data.local,
		&email_verification_data.domain_id,
	)
	.await?;

	Ok(())
}

pub async fn change_password_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	old_password: &str,
	new_password: &str,
) -> Result<(), Error> {
	let user = db::get_user_by_user_id(connection, user_id)
		.await?
		.status(500)
		.body(error!(USER_NOT_FOUND).to_string())?;

	let success = service::validate_hash(old_password, &user.password)?;

	if !success {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_PASSWORD).to_string())?;
	}

	let new_password = service::hash(new_password.as_bytes())?;

	db::update_user_password(connection, &user_id, &new_password).await?;

	Ok(())
}
