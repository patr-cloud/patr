use eve_rs::AsError;

use crate::{
	db,
	error,
	service,
	utils::{get_current_time_millis, validator, Error},
	Database,
};
use crate::models::db_mapping::UserPhoneNumber;

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

pub async fn get_user_emails(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8]
) -> Result<(Vec<String>, Vec<String>), Error> {

	let personal_email_list =
		db::get_personal_emails(
			connection,
			&user_id
		)
		.await?;

	let mut personal_email =  Vec::new();

	for email in personal_email_list {
		personal_email.push(email.personal_email);
	}

	let organisation_email_list =
		db::get_organisation_emails(
			connection,
			&user_id
		)
		.await?;
	
	let mut organisation_email = Vec::new();

	for email in organisation_email_list {
		organisation_email.push(email.organisation_email);
	}

	Ok((personal_email, organisation_email))
}

pub async fn get_user_phone_numbers(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8]
) -> Result<Vec<UserPhoneNumber>, Error> {
	let phone_numbers =
		db::get_user_phone_numbers(
			connection,
			&user_id
		)
		.await?
		.into_iter()
		.map( |phone_number| UserPhoneNumber {
			user_id: phone_number.user_id,
			country_code: phone_number.country_code,
			number: phone_number.number
		})
		.collect::<Vec<_>>();

	Ok(phone_numbers)
}

pub async fn update_user_backup_email(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	email_address: &str
) -> Result<(), Error> {
	if !validator::is_email_valid(&email_address) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_EMAIL).to_string())?;
	}
	// split email into 2 parts
	let (email_local, domain_name) = email_address
		.split_once('@')
		.status(400)
		.body(error!(INVALID_EMAIL).to_string())?;
	// check if the email domain exists
	let personal_domain = db::get_domain_by_name(
		connection,
		domain_name
	)
	.await?;

	// if domain doesn't exists then return an error
	if personal_domain.is_none() {
		Error::as_result()
			.status(400)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?
	}

	// safe unwrap
	let personal_domain = personal_domain.unwrap();

	// check if the email exists under user's id
	let personal_email = db::get_personal_email(
		connection,
		user_id,
		&personal_domain.name,
		&personal_domain.id
	)
	.await?;

	// if it is None then the email isn't registered under user's id
	if personal_email.is_none() {
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_NOT_FOUND).to_string())?
	}

	// finally if everything checks out then change the personal email
    db::update_user_backup_email(
        connection,
        &user_id,
        &email_local,
        &personal_domain.id
    )
    .await?;

	Ok(())
}

pub async fn update_user_backup_phone_number(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	country_code: &str,
	phone_number: &str
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

    let user_phone_details = db::get_user_phone_number(
        connection,
        &user_id,
        country_code,
        phone_number
	)
	.await?;

	if user_phone_details.is_none() {
		Error::as_result()
			.status(400)
			.body(error!(PHONE_NUMBER_NOT_FOUND).to_string())?;
	}

	db::update_user_backup_phone_number(
		connection,
		&user_id,
		&country_code,
		&phone_number
	)
		.await?;

	Ok(())
}