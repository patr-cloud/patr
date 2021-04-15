use eve_rs::AsError;
use serde_json::Value;
use sqlx::{MySql, Transaction};
use tokio::task;
use utils::{get_current_time, mailer};
use uuid::Uuid;

use crate::{
	db,
	error,
	models::{
		db_mapping::{User, UserEmailAddress, UserEmailAddressSignUp},
		rbac,
		AccessTokenData,
		ExposedUserData,
	},
	service,
	utils::{
		self,
		constants::AccountType,
		settings::Settings,
		validator,
		AsErrorData,
		EveError as Error,
	},
};

pub async fn is_username_allowed(
	transaction: &mut Transaction<'_, MySql>,
	username: &str,
) -> Result<bool, Error> {
	if !validator::is_username_valid(&username) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_USERNAME).to_string())?;
	}
	db::get_user_by_username(transaction, username)
		.await
		.map(|user| user.is_none())
		.status(500)
		.body(error!(SERVER_ERROR).to_string())
}

pub async fn is_email_allowed(
	transaction: &mut Transaction<'_, MySql>,
	email: &str,
) -> Result<bool, Error> {
	if !validator::is_email_valid(&email) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_EMAIL).to_string())?;
	}

	db::get_user_by_email(transaction, email)
		.await
		.map(|user| user.is_none())
		.status(500)
		.body(error!(SERVER_ERROR).to_string())
}

/// this function creates a new user to be signed up and returns a OTP
pub async fn create_user_join_request(
	transaction: &mut Transaction<'_, MySql>,
	config: &Settings,
	username: &str,
	email: &str,
	password: &str,
	account_type: AccountType,
	(first_name, last_name): (&str, &str),
	(domain_name, organisation_name, backup_email): (
		Option<&str>,
		Option<&str>,
		Option<&str>,
	),
) -> Result<String, Error> {
	if !is_username_allowed(transaction, username).await? {
		return None.status(200).body(error!(USERNAME_TAKEN).to_string());
	}

	if !is_email_allowed(transaction, email).await? {
		return None.status(200).body(error!(EMAIL_TAKEN).to_string());
	}

	if let Some(backup_email) = backup_email {
		if !validator::is_email_valid(backup_email) {
			return None.status(200).body(error!(INVALID_EMAIL).to_string());
		}
	}

	if !validator::is_password_valid(&password) {
		return None.status(200).body(error!(PASSWORD_TOO_WEAK).to_string());
	}

	if let Some(domain) = domain_name {
		if !validator::is_domain_name_valid(domain).await {
			return None
				.status(200)
				.body(error!(INVALID_DOMAIN_NAME).to_string());
		}
	}

	let otp = service::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);
	let token_expiry = get_current_time() + (1000 * 60 * 60 * 2); // 2 hours

	let password =
		service::hash(password.as_bytes(), config.password_salt.as_bytes())?;
	let token_hash =
		service::hash(otp.as_bytes(), config.password_salt.as_bytes())?;

	let email = match account_type {
		AccountType::Organisation => {
			let domain_name = domain_name.unwrap();
			if !email.ends_with(&format!("@{}", domain_name)) {
				return None
					.status(200)
					.body(error!(INVALID_EMAIL).to_string());
			}
			UserEmailAddressSignUp::Organisation {
				email_local: email.replace(&format!("@{}", domain_name), ""),
				domain_name: domain_name.to_string(),
				organisation_name: organisation_name.unwrap().to_string(),
				backup_email: backup_email.unwrap().to_string(),
			}
		}
		AccountType::Personal => {
			UserEmailAddressSignUp::Personal(email.to_string())
		}
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
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	Ok(otp)
}

/// function to sign in a user
/// Returns: JWT (String), Refresh Token (Uuid)
pub async fn sign_in(
	transaction: &mut Transaction<'_, MySql>,
	user: User,
	config: &Settings,
) -> Result<(String, Uuid), Error> {
	// generate JWT
	let iat = get_current_time();
	let exp = iat + (1000 * 3600 * 24 * 3); // 3 days
	let orgs = db::get_all_organisation_roles_for_user(transaction, &user.id)
		.await
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let user = ExposedUserData {
		id: user.id,
		username: user.username,
		first_name: user.first_name,
		last_name: user.last_name,
		created: user.created,
	};

	let refresh_token = Uuid::new_v4();

	db::add_user_login(
		transaction,
		refresh_token.as_bytes(),
		iat + (1000 * 60 * 60 * 24 * 30), // 30 days
		&user.id,
		iat,
		iat,
	)
	.await
	.status(500)
	.body(error!(SERVER_ERROR).to_string())
	.commit_transaction(false)?;

	let token_data = AccessTokenData::new(iat, exp, orgs, user);
	let jwt = token_data
		.to_string(config.jwt_secret.as_str())
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	Ok((jwt, refresh_token))
}

pub async fn get_access_token_data(
	transaction: &mut Transaction<'_, MySql>,
	config: Settings,
	refresh_token: &str,
) -> Result<String, Error> {
	let refresh_token = if let Ok(uuid) = Uuid::parse_str(&refresh_token) {
		uuid
	} else {
		return Err(error!(WRONG_PARAMETERS));
	};
	let refresh_token = refresh_token.as_bytes();

	let user_login = db::get_user_login(transaction, refresh_token).await?;

	if user_login.is_none() {
		// context.json(error!(EMAIL_TOKEN_NOT_FOUND));
		return Err(error!(EMAIL_TOKEN_NOT_FOUND));
	}
	let user_login = user_login.unwrap();

	if user_login.token_expiry < get_current_time() {
		// Token has expired
		return Err(error!(EXPIRED));
	}

	// get roles and permissions of user for rbac here
	// use that info to populate the data in the token_data

	let iat = get_current_time();
	let exp = iat + (1000 * 60 * 60 * 24 * 3); // 3 days
	let orgs = db::get_all_organisation_roles_for_user(
		transaction,
		&user_login.user_id,
	)
	.await?;

	let user_id = user_login.user_id;
	let user_data = db::get_user_by_user_id(transaction, &user_id)
		.await?
		.unwrap();

	let user = ExposedUserData {
		id: user_id,
		username: user_data.username,
		first_name: user_data.first_name,
		last_name: user_data.last_name,
		created: user_data.created,
	};

	let token_data = AccessTokenData::new(iat, exp, orgs, user);
	let jwt = token_data
		.to_string(config.jwt_secret.as_str())
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	db::set_refresh_token_expiry(transaction, refresh_token, iat, exp).await?;

	Ok(jwt)
}

// function to reset password
// TODO: Remove otp from response
pub async fn forgot_password(
	transaction: &mut Transaction<'_, MySql>,
	config: Settings,
	user_id: &str,
) -> Result<String, Value> {
	let user = db::get_user_by_username_or_email(transaction, &user_id)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;
	let user = user.unwrap();

	let otp = service::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry = get_current_time() + (1000 * 60 * 60 * 2); // 2 hours

	let token_hash =
		service::hash(otp.as_bytes(), config.password_salt.as_bytes())
			.map_err(|_| error!(SERVER_ERROR))?;

	db::add_password_reset_request(
		transaction,
		&user.id,
		&token_hash,
		token_expiry,
	)
	.await
	.map_err(|_| error!(SERVER_ERROR))?;
	let otp_clone = otp.clone();
	task::spawn_blocking(|| {
		mailer::send_password_reset_requested_mail(
			config,
			user.backup_email,
			otp,
		);
	});

	Ok(otp_clone)
}

pub async fn reset_password(
	transaction: &mut Transaction<'_, MySql>,
	config: &Settings,
	// pool: Pool<MySql>,
	new_password: &str,
	token: &str,
	user_id: &[u8],
) -> Result<(), Value> {
	let reset_request =
		db::get_password_reset_request_for_user(transaction, &user_id)
			.await
			.map_err(|_| error!(SERVER_ERROR))?;

	if reset_request.is_none() {
		// context.status(400).json(error!(EMAIL_TOKEN_NOT_FOUND));
		return Err(error!(EMAIL_TOKEN_NOT_FOUND));
	}
	let reset_request = reset_request.unwrap();

	let success = service::validate_hash(
		token.as_bytes(),
		config.password_salt.as_bytes(),
		&reset_request.token,
	)
	.map_err(|_| error!(SERVER_ERROR))?;

	if !success {
		// context.status(400).json(error!(EMAIL_TOKEN_NOT_FOUND));
		// return Ok(context);
		return Err(error!(EMAIL_TOKEN_NOT_FOUND));
	}

	let new_password =
		service::hash(new_password.as_bytes(), config.password_salt.as_bytes())
			.map_err(|_| error!(SERVER_ERROR))?;

	db::update_user_password(transaction, &user_id, &new_password)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	db::delete_password_reset_request_for_user(transaction, &user_id)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	Ok(())
}

pub async fn join(
	transaction: &mut Transaction<'_, MySql>,
	config: Settings,
	otp: &str,
	username: &str,
) -> Result<(String, Uuid, String, Option<String>), Value> {
	let user_data = if let Some(user_data) =
		db::get_user_email_to_sign_up(transaction, &username)
			.await
			.map_err(|_| error!(SERVER_ERROR))?
	{
		user_data
	} else {
		return Err(error!(INVALID_OTP));
	};

	let success = service::validate_hash(
		otp.as_bytes(),
		config.password_salt.as_bytes(),
		&user_data.otp_hash,
	)
	.map_err(|_| error!(SERVER_ERROR))?;

	if !success {
		return Err(error!(INVALID_OTP));
	}

	if user_data.otp_expiry < get_current_time() {
		return Err(error!(OTP_EXPIRED));
	}

	// For a personal account, get:
	// - username
	// - email
	// - password
	// - account_type
	// - first_name
	// - last_name

	// For an organisation account, also get:
	// - domain_name
	// - organisation_name
	// - backup_email

	// First create user,
	// Then create an organisation if an org account,
	// Then add the domain if org account,
	// Then create personal org regardless,
	// Then set email to backup email if personal account,
	// And finally send the token, along with the email to the user

	let user_uuid = Uuid::new_v4();
	let user_id = user_uuid.as_bytes();
	let created = get_current_time();

	if rbac::GOD_USER_ID.get().is_none() {
		rbac::GOD_USER_ID
			.set(user_uuid)
			.expect("GOD_USER_ID was already set");
	}

	db::create_user(
		transaction,
		user_id,
		&user_data.username,
		&user_data.password,
		&user_data.backup_email,
		(&user_data.first_name, &user_data.last_name),
		created,
	)
	.await
	.map_err(|_| error!(SERVER_ERROR))?;

	// For an organisation, create the organisation and domain
	let email;
	let welcome_email_to;
	let backup_email_notification_to;
	match user_data.email {
		UserEmailAddressSignUp::Personal(email_address) => {
			email = UserEmailAddress::Personal(email_address.clone());
			backup_email_notification_to = None;
			welcome_email_to = email_address;
		}
		UserEmailAddressSignUp::Organisation {
			domain_name,
			email_local,
			backup_email,
			organisation_name,
		} => {
			let organisation_id = db::generate_new_resource_id(transaction)
				.await
				.map_err(|_| error!(SERVER_ERROR))?;
			let organisation_id = organisation_id.as_bytes();
			db::create_orphaned_resource(
				transaction,
				organisation_id,
				&format!("Organiation: {}", organisation_name),
				rbac::RESOURCE_TYPES
					.get()
					.unwrap()
					.get(rbac::resource_types::ORGANISATION)
					.unwrap(),
			)
			.await
			.map_err(|_| error!(SERVER_ERROR))?;
			db::create_organisation(
				transaction,
				organisation_id,
				&organisation_name,
				user_id,
				get_current_time(),
			)
			.await
			.map_err(|_| error!(SERVER_ERROR))?;
			db::set_resource_owner_id(
				transaction,
				organisation_id,
				organisation_id,
			)
			.await
			.map_err(|_| error!(SERVER_ERROR))?;

			let domain_id = db::generate_new_resource_id(transaction)
				.await
				.map_err(|_| error!(SERVER_ERROR))?;
			let domain_id = domain_id.as_bytes().to_vec();

			db::create_resource(
				transaction,
				&domain_id,
				&format!("Domain: {}", domain_name),
				rbac::RESOURCE_TYPES
					.get()
					.unwrap()
					.get(rbac::resource_types::DOMAIN)
					.unwrap(),
				organisation_id,
			)
			.await
			.map_err(|_| error!(SERVER_ERROR))?;
			db::add_domain_to_organisation(
				transaction,
				&domain_id,
				&domain_name,
			)
			.await
			.map_err(|_| error!(SERVER_ERROR))?;

			welcome_email_to = format!("{}@{}", email_local, domain_name);
			email = UserEmailAddress::Organisation {
				domain_id,
				email_local,
			};
			backup_email_notification_to = Some(backup_email);
		}
	}

	// add personal organisation
	let organisation_id = db::generate_new_resource_id(transaction)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;
	let organisation_id = organisation_id.as_bytes();
	let organisation_name =
		format!("personal-organisation-{}", hex::encode(user_id));

	db::create_orphaned_resource(
		transaction,
		organisation_id,
		&organisation_name,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::ORGANISATION)
			.unwrap(),
	)
	.await
	.map_err(|_| error!(SERVER_ERROR))?;

	db::create_organisation(
		transaction,
		organisation_id,
		&organisation_name,
		user_id,
		get_current_time(),
	)
	.await
	.map_err(|_| error!(SERVER_ERROR))?;
	db::set_resource_owner_id(transaction, organisation_id, organisation_id)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	db::add_email_for_user(transaction, user_id, email)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;
	db::delete_user_to_be_signed_up(transaction, &user_data.username)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	let user = if let Some(user) =
		db::get_user_by_username_or_email(transaction, &user_data.username)
			.await
			.map_err(|_| error!(SERVER_ERROR))?
	{
		user
	} else {
		return Err(error!(USER_NOT_FOUND));
	};

	let (jwt, refresh_token) = sign_in(transaction, user, &config)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	Ok((
		jwt,
		refresh_token,
		welcome_email_to,
		backup_email_notification_to,
	))
}
