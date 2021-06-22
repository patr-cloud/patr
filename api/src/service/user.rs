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
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry =
		get_current_time_millis() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	// split email into 2 parts and get domain_id
	let (email_local, personal_domain_id) =
		service::split_email_with_domain_id(connection, email_address).await?;

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

	// delete from table user_unverified_personal_email after verification
	db::delete_user_unverified_personal_email(
		connection, 
		&user_id, 
		&email_verification_data.local, 
		&email_verification_data.domain_id,
	)
	.await?;

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
	let (email_local, domain_id) =
		service::split_email_with_domain_id(connection, email_address).await?;

	// finally if everything checks out then change the personal email
	db::update_backup_email_for_user(
		connection,
		&user_id,
		&email_local,
		&domain_id,
	)
	.await
	.status(400)
	.body(error!(INVALID_EMAIL).to_string())?;

	Ok(())
}

pub async fn update_user_backup_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	country_code: &str,
	phone_number: &str,
) -> Result<(), Error> {
	if !validator::is_phone_number_valid(phone_number) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_PHONE_NUMBER).to_string())?;
	}

	let country_code =
		db::get_phone_country_by_country_code(connection, country_code)
			.await?
			.status(400)
			.body(error!(INVALID_COUNTRY_CODE).to_string())?;

	db::update_backup_phone_number_for_user(
		connection,
		&user_id,
		&country_code.country_code,
		&phone_number,
	)
	.await
	.status(400)
	.body(error!(INVALID_PHONE_NUMBER).to_string())?;

	Ok(())
}

pub async fn delete_personal_email_address(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	email_address: &str,
) -> Result<(), Error> {
	let (email_local, domain_id) =
		service::split_email_with_domain_id(connection, email_address).await?;

	let user_data = db::get_user_by_user_id(connection, &user_id).await?;

	let user_data = if let Some(user_data) = user_data {
		user_data
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	};

	if let Some((backup_email_local, backup_domain_id)) = user_data
		.backup_email_local
		.zip(user_data.backup_email_domain_id)
	{
		if backup_email_local == email_local && backup_domain_id == domain_id {
			return Error::as_result()
				.status(400)
				.body(error!(CANNOT_DELETE_BACKUP_EMAIL).to_string())?;
		}
	}

	db::delete_personal_email_for_user(
		connection,
		&user_id,
		&email_local,
		&domain_id,
	)
	.await?;

	//TODO: add function to find if domain_id is used by any personal email or not, if not then add a delete function
	// Would be a lot easier if we use on delete cascade
	let personal_email_list = db::get_personal_emails_by_domain_id(
		connection, 
		&domain_id
	)
	.await?;

	if personal_email_list.is_empty() {
		// first delete from personal domain
		db::delete_personal_domain(
			connection, 
			&domain_id
		)
		.await?;

		// then from the main domain table
		db::delete_generic_domain(
			connection, 
			&domain_id
		)
		.await?;
	}

	Ok(())
}

pub async fn delete_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	country_code: &str,
	phone_number: &str,
) -> Result<(), Error> {
	let user_data = db::get_user_by_user_id(connection, &user_id).await?;

	let user_data = if let Some(user_data) = user_data {
		user_data
	} else {
		return Err(Error::empty()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string()));
	};

	if let Some((backup_country_code, backup_phone_number)) = user_data
		.backup_phone_country_code
		.zip(user_data.backup_phone_number)
	{
		if backup_country_code == country_code &&
			backup_phone_number == phone_number
		{
			return Error::as_result()
				.status(400)
				.body(error!(CANNOT_DELETE_BACKUP_PHONE_NUMBER).to_string())?;
		}
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

pub async fn add_phone_number_to_be_verified_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	country_code: &str,
	phone_number: &str,
) -> Result<String, Error> {
	if !service::is_phone_number_allowed(connection, country_code, phone_number)
		.await?
	{
		Error::as_result()
			.status(400)
			.body(error!(PHONE_NUMBER_TAKEN).to_string())?;
	}

	let otp = service::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry =
		get_current_time_millis() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	db::add_phone_number_to_be_verified_for_user(
		connection,
		&country_code,
		&phone_number,
		&user_id,
		&verification_token,
		token_expiry,
	)
	.await?;

	Ok(otp)
}

pub async fn verify_phone_number_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	country_code: &str,
	phone_number: &str,
	otp: &str,
) -> Result<(), Error> {
	let phone_verification_data = db::get_phone_number_to_be_verified_for_user(
		connection,
		user_id,
		country_code,
		phone_number,
	)
	.await?
	.status(400)
	.body(error!(PHONE_NUMBER_TOKEN_EXPIRED).to_string())?;

	let success = service::validate_hash(
		otp,
		&phone_verification_data.verification_token_hash,
	)?;

	if phone_verification_data.verification_token_expiry <
		get_current_time_millis()
	{
		Error::as_result()
			.status(200)
			.body(error!(PHONE_NUMBER_TOKEN_EXPIRED).to_string())?;
	}

	if !success {
		Error::as_result()
			.status(400)
			.body(error!(PHONE_NUMBER_TOKEN_NOT_FOUND).to_string())?;
	}

	// delete from table user_unverified_phone_number after verification
	db::delete_user_unverified_phone_number(
		connection, 
		&user_id, 
		&phone_verification_data.country_code,
		&phone_verification_data.phone_number,
	)
	.await?;

	db::add_phone_number_for_user(
		connection,
		user_id,
		&phone_verification_data.country_code,
		&phone_verification_data.phone_number,
	)
	.await?;

	Ok(())
}
