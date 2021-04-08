use std::collections::HashMap;

use crate::{
	db, error,
	models::{
		db_mapping::{User, UserEmailAddressSignUp},
		rbac::OrgPermissions,
		AccessTokenData, ExposedUserData,
	},
	utils::{self, settings::Settings, validator},
};
use argon2::{Error, Variant};
use jsonwebtoken::errors::Error as JWTError;
use serde_json::Value;
use sqlx::{MySql, Transaction};
use utils::get_current_time;
use uuid::Uuid;

pub fn verify_hash(
	pwd: &[u8],
	salt: &[u8],
	otp_hash: &[u8],
) -> Result<bool, Error> {
	argon2::verify_raw(
		pwd,
		salt,
		otp_hash,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)
}

/// function to get token hash
pub fn hash(pwd: &[u8], salt: &[u8]) -> Result<Vec<u8>, Error> {
	argon2::hash_raw(
		pwd,
		salt,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)
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

/// this function creates a new user to be signed up and returns a OTP
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

	if backup_email.is_some()
		&& !validator::is_email_valid(backup_email.as_ref().unwrap())
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

/// function to create a jwt token and return it's string value
pub fn get_jwt_token(
	iat: u64,
	exp: u64,
	orgs: HashMap<String, OrgPermissions>,
	user: ExposedUserData,
	config: &Settings,
) -> Result<String, JWTError> {
	let token_data = AccessTokenData::new(iat, exp, orgs, user);
	token_data.to_string(config.jwt_secret.as_str())
}

/// function to sign in a user
/// Returns: JWT (String), Refresh Token (Uuid)
pub async fn sign_in(
	transaction: &mut Transaction<'_, MySql>,
	user: User,
	config: Settings,
) -> Result<(String, Uuid), Value> {
	// generate JWT
	let iat = get_current_time();
	let exp = iat + (1000 * 3600 * 24 * 3); // 3 days
	let orgs =
		db::get_all_organisation_roles_for_user(transaction, &user.id).await;

	// return server error.
	if let Err(_) = orgs {
		return Err(error!(SERVER_ERROR));
	}
	let orgs = orgs.unwrap();

	let user = ExposedUserData {
		id: user.id,
		username: user.username,
		first_name: user.first_name,
		last_name: user.last_name,
		created: user.created,
	};

	let jwt = get_jwt_token(iat, exp, orgs, user.clone(), &config);
	if let Err(_) = jwt {
		return Err(error!(SERVER_ERROR));
	}
	let jwt = jwt.unwrap();

	let refresh_token = Uuid::new_v4();

	let add_user_login_result = db::add_user_login(
		transaction,
		refresh_token.as_bytes(),
		iat + (1000 * 60 * 60 * 24 * 30), // 30 days
		&user.id,
		iat,
		iat,
	)
	.await;
	if let Err(err) = add_user_login_result {
		return Err(error!(SERVER_ERROR));
	}

	Ok((jwt, refresh_token))
}
