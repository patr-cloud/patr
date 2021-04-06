use crate::{
	db,
	error,
	models::db_mapping::UserEmailAddressSignUp,
	utils::{self, settings::Settings, validator},
};
use argon2::{Error, Variant};
use serde_json::Value;
use sqlx::{MySql, Transaction};
use utils::get_current_time;

/// function to get token hash
pub fn hash(pwd: &[u8], salt: &[u8]) -> Result<Vec<u8>, Error> {
	return argon2::hash_raw(
		pwd,
		salt,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	);
}

pub async fn is_username_allowed(
	transaction: &mut Transaction<'_, MySql>,
	username: &str,
) -> Result<(), Value> {
	if !validator::is_username_valid(&username) {
		return Err(error!(INVALID_USERNAME));
	}
	db::get_user_by_username(transaction, username)
		.await
		.map_err(|_| error!(SERVER_ERROR))
		.map(|_| ())
}

pub async fn is_email_allowed(
	transaction: &mut Transaction<'_, MySql>,
	email: &str,
) -> Result<(), Value> {
	if !validator::is_email_valid(&email) {
		return Err(error!(INVALID_EMAIL));
	}

	db::get_user_by_email(transaction, email)
		.await
		.map_err(|_| error!(SERVER_ERROR))
		.map(|_| ())
}

/// function to create new user
pub async fn create_user_to_be_signed_up(
	transaction: &mut Transaction<'_, MySql>,
	config: &Settings,
	username: &String,
	email: &String,
	password: &String,
	account_type: &String,
	domain_name: Option<&String>,
	organisation_name: Option<&String>,
	backup_email: Option<&String>,
	first_name: &String,
	last_name: &String,
) -> Result<String, Value> {
	is_username_allowed(transaction, username).await?;
	is_email_allowed(transaction, email).await?;

	if backup_email.is_some() &&
		!validator::is_email_valid(backup_email.as_ref().unwrap())
	{
		return Err(error!(INVALID_EMAIL));
	}

	if !validator::is_password_valid(&password) {
		return Err(error!(PASSWORD_TOO_WEAK));
	}

	if let Some(domain) = domain_name {
		if !validator::is_domain_name_valid(domain).await {
			return Err(error!(INVALID_DOMAIN_NAME));
		}
	}

	let otp = utils::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);
	let token_expiry = get_current_time() + (1000 * 60 * 60 * 2); // 2 hours

	let password = hash(password.as_bytes(), config.password_salt.as_bytes())
		.map_err(|_| error!(SERVER_ERROR))?;

	let token_hash = hash(otp.as_bytes(), config.password_salt.as_bytes())
		.map_err(|_| error!(SERVER_ERROR))?;

	let email = if account_type == "organisation" {
		UserEmailAddressSignUp::Organisation {
			email_local: email
				.replace(&format!("@{}", domain_name.unwrap()), ""),
			domain_name: domain_name.unwrap().to_string().clone(),
			organisation_name: organisation_name.unwrap().to_string().clone(),
			backup_email: backup_email.unwrap().to_string().clone(),
		}
	} else if account_type == "personal" {
		UserEmailAddressSignUp::Personal(email.to_string().clone())
	} else {
		panic!("email type is neither personal, nor organisation. How did you even get here?")
	};

	db::set_user_to_be_signed_up(
		transaction,
		email.clone(),
		&username,
		&password,
		(&first_name, &last_name),
		&token_hash,
		token_expiry,
	)
	.await
	.map_err(|_| error!(SERVER_ERROR))?;

	Ok(otp)
}
