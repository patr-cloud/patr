use eve_rs::AsError;

use crate::{
	db,
	error,
	models::db_mapping::UserPhoneNumber,
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
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry =
		get_current_time_millis() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	// split email into 2 parts and get domain_id
	let (email_local, personal_domain_id) =
		service::get_local_and_domain_id_from_email(
			connection,
			email_address,
			true,
		)
		.await?;

	db::add_personal_email_to_be_verified_for_user(
		connection,
		&email_local,
		&personal_domain_id,
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

pub async fn get_personal_emails_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
) -> Result<Vec<String>, Error> {
	let personal_email: Vec<String> =
		db::get_personal_emails_for_user(connection, &user_id).await?;

	Ok(personal_email)
}

pub async fn get_phone_numbers_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
) -> Result<Vec<UserPhoneNumber>, Error> {
	let phone_numbers =
		db::get_phone_numbers_for_users(connection, &user_id).await?;

	Ok(phone_numbers)
}

pub async fn update_user_backup_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	email_address: &str,
) -> Result<(), Error> {
	if !validator::is_email_valid(&email_address) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_EMAIL).to_string())?;
	}
	// split email into 2 parts and get domain_id
	let (email_local, domain_id) = service::get_local_and_domain_id_from_email(
		connection,
		email_address,
		false,
	)
	.await?;

	// check if the email exists under user's id
	let personal_email =
		db::check_if_personal_email_exists(connection, user_id).await?;

	// if it is false then the email isn't registered under user's id
	if !personal_email {
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_NOT_FOUND).to_string())?
	}

	// finally if everything checks out then change the personal email
	db::update_backup_email_for_user(
		connection,
		&user_id,
		&email_local,
		&domain_id,
	)
	.await?;

	Ok(())
}

pub async fn update_user_backup_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	country_code: &str,
	phone_number: &str,
) -> Result<(), Error> {
	if !validator::is_country_code_valid(country_code) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_COUNTRY_CODE).to_string())?;
	}

	if !validator::is_phone_number_valid(&phone_number) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_PHONE_NUMBER).to_string())?;
	}

	let user_phone_details = db::check_if_personal_phone_number_exists(
		connection,
		&user_id,
		country_code,
		phone_number,
	)
	.await?;

	// if is false then phone number doesn't exist under user's id
	if !user_phone_details {
		Error::as_result()
			.status(400)
			.body(error!(PHONE_NUMBER_NOT_FOUND).to_string())?;
	}

	db::update_backup_phone_number_for_user(
		connection,
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;

	Ok(())
}

pub async fn delete_personal_email_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	email_address: &str,
) -> Result<(), Error> {
	let (email_local, domain_id) = service::get_local_and_domain_id_from_email(
		connection,
		email_address,
		false,
	)
	.await?;

	let user_data = db::get_user_by_user_id(connection, &user_id).await?;

	if user_data.is_none() {
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	// safe unwrap
	let user_data = user_data.unwrap();

	let backup_email_local = user_data.backup_email_local;
	let backup_email_domain = user_data.backup_email_domain_id;

	if backup_email_local.is_some() {
		let backup_email_local = backup_email_local.unwrap();
		let backup_email_domain = backup_email_domain.unwrap();

		if backup_email_local == email_local && backup_email_domain == domain_id
		{
			Error::as_result()
				.status(400)
				.body(error!(CANNOT_DELETE_BACKUP_EMAIL).to_string())?;
		}
	}

	// if backup_email.is_some() {
	// Error::as_result()
	// 	.status(400)
	// 	.body(error!(CANNOT_DELETE_BACKUP_EMAIL).to_string())?;
	// }

	db::delete_personal_email_for_user(
		connection,
		&user_id,
		&email_local,
		&domain_id,
	)
	.await?;

	Ok(())
}

pub async fn delete_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	country_code: &str,
	phone_number: &str,
) -> Result<(), Error> {
	let backup_phone_number = db::get_backup_phone_number_from_user(
		connection,
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;

	if backup_phone_number.is_some() {
		Error::as_result()
			.status(400)
			.body(error!(CANNOT_DELETE_BACKUP_PHONE_NUMBER).to_string())?;
	}

	db::delete_phone_number_for_user(
		connection,
		&user_id,
		&country_code,
		&phone_number,
	)
	.await?;

	Ok(())
}
