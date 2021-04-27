use eve_rs::AsError;
use sqlx::{MySql, Transaction};

use crate::{
	db,
	error,
	models::db_mapping::UserEmailAddress,
	service,
	utils::{get_current_time, validator, Error},
};

pub async fn add_personal_email_to_be_verified_for_user(
	connection: &mut Transaction<'_, MySql>,
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

	let token_expiry = get_current_time() + service::get_join_token_expiry();
	let verification_token = service::hash(otp.as_bytes())?;

	db::add_personal_email_to_be_verified_for_user(
		connection,
		&email_address,
		&user_id,
		&verification_token,
		token_expiry,
	)
	.await?;

	Ok(())
}

pub async fn verify_personal_email_address_for_user(
	connection: &mut Transaction<'_, MySql>,
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

	if email_verification_data.verification_token_expiry < get_current_time() {
		Error::as_result()
			.status(200)
			.body(error!(EMAIL_TOKEN_EXPIRED).to_string())?;
	}

	let email_domain_local: Vec<&str> = email_verification_data.email_address.split('@').collect();

	let domain_id = db::get_domain_by_name(
		connection, 
		email_domain_local[1]
	)
	.await?
	.status(404)
	.body(error!(INVALID_DOMAIN_NAME).to_string())?;

	let email_address =
		UserEmailAddress::Personal{
			email: email_verification_data.email_address,
			domain_id: domain_id.id
		};

	db::add_email_for_user(connection, user_id, email_address).await?;

	Ok(())
}
