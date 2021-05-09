use eve_rs::AsError;
use sqlx::{MySql, Transaction};
use tokio::task;
use uuid::Uuid;

use super::get_refresh_token_expiry;
use crate::{
	db, error,
	models::{
		db_mapping::{
			PersonalDomain, User, UserEmailAddress, UserEmailAddressSignUp,
			UserLogin,
		},
		rbac, AccessTokenData, ExposedUserData,
	},
	service,
	utils::{
		constants::AccountType, get_current_time, mailer, settings::Settings,
		validator, Error,
	},
};

pub async fn is_username_allowed(
	connection: &mut Transaction<'_, MySql>,
	username: &str,
) -> Result<bool, Error> {
	if !validator::is_username_valid(&username) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_USERNAME).to_string())?;
	}
	db::get_user_by_username(connection, username)
		.await
		.map(|user| user.is_none())
		.status(500)
}

pub async fn is_email_allowed(
	connection: &mut Transaction<'_, MySql>,
	email: &str,
) -> Result<bool, Error> {
	if !validator::is_email_valid(&email) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_EMAIL).to_string())?;
	}

	db::get_user_by_email(connection, email)
		.await
		.map(|user| user.is_none())
		.status(500)
}

/// this function creates a new user to be signed up and returns a OTP
pub async fn create_user_join_request(
	connection: &mut Transaction<'_, MySql>,
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
	if !is_username_allowed(connection, username).await? {
		Error::as_result()
			.status(200)
			.body(error!(USERNAME_TAKEN).to_string())?;
	}

	if !is_email_allowed(connection, email).await? {
		Error::as_result()
			.status(200)
			.body(error!(EMAIL_TAKEN).to_string())?;
	}

	if !validator::is_password_valid(&password) {
		Error::as_result()
			.status(200)
			.body(error!(PASSWORD_TOO_WEAK).to_string())?;
	}

	let email = match account_type {
		AccountType::Organisation => {
			let domain_name = domain_name
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?
				.to_string();
			let organisation_name = organisation_name
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?
				.to_string();
			let backup_email = backup_email
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?
				.to_string();

			if !validator::is_domain_name_valid(&domain_name).await {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_DOMAIN_NAME).to_string())?;
			}

			if !validator::is_organisation_name_valid(&organisation_name) {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_ORGANISATION_NAME).to_string())?;
			}

			if db::get_organisation_by_name(connection, &organisation_name)
				.await?
				.is_some()
			{
				Error::as_result()
					.status(200)
					.body(error!(ORGANISATION_EXISTS).to_string())?;
			}

			if db::get_user_to_sign_up_by_organisation_name(
				connection,
				&organisation_name,
			)
			.await?
			.is_some()
			{
				Error::as_result()
					.status(200)
					.body(error!(ORGANISATION_EXISTS).to_string())?;
			}

			if !validator::is_email_valid(&backup_email) {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_EMAIL).to_string())?;
			}

			if !email.ends_with(&format!("@{}", domain_name)) {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_EMAIL).to_string())?;
			}
			UserEmailAddressSignUp::Organisation {
				email_local: email.replace(&format!("@{}", domain_name), ""),
				domain_name,
				organisation_name,
				backup_email,
			}
		}
		AccountType::Personal => {
			UserEmailAddressSignUp::Personal(email.to_string())
		}
	};

	let otp = service::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);
	let token_expiry = get_current_time() + service::get_join_token_expiry();

	let password = service::hash(password.as_bytes())?;
	let token_hash = service::hash(otp.as_bytes())?;

	db::set_user_to_be_signed_up(
		connection,
		email,
		&username,
		&password,
		(&first_name, &last_name),
		&token_hash,
		token_expiry,
	)
	.await?;

	Ok(otp)
}

// Creates a login in the db and returns it
// loginId and refresh_token are separate things
pub async fn create_login_for_user(
	connection: &mut Transaction<'_, MySql>,
	user_id: &[u8],
) -> Result<UserLogin, Error> {
	let login_id = db::generate_new_login_id(connection).await?;
	let (refresh_token, hashed_refresh_token) =
		service::generate_new_refresh_token_for_user(connection, user_id)
			.await?;
	let iat = get_current_time();

	db::add_user_login(
		connection,
		login_id.as_bytes(),
		&hashed_refresh_token,
		iat + get_refresh_token_expiry(),
		user_id,
		iat,
		iat,
	)
	.await?;
	let user_login = UserLogin {
		login_id: login_id.as_bytes().to_vec(),
		user_id: user_id.to_vec(),
		last_activity: iat,
		last_login: iat,
		refresh_token,
		token_expiry: iat + get_refresh_token_expiry(),
	};

	Ok(user_login)
}

/// function to sign in a user
/// Returns: JWT (String), Refresh Token (Uuid)
pub async fn sign_in_user(
	connection: &mut Transaction<'_, MySql>,
	user: User,
	config: &Settings,
) -> Result<(String, Uuid), Error> {
	let refresh_token = Uuid::new_v4();

	let user_login = create_login_for_user(connection, &user.id).await?;

	let jwt = generate_access_token(connection, &config, &user_login).await?;

	Ok((jwt, refresh_token))
}

pub async fn get_user_login_for_login_id(
	connection: &mut Transaction<'_, MySql>,
	login_id: &[u8],
) -> Result<UserLogin, Error> {
	let user_login = db::get_user_login(connection, login_id)
		.await?
		.status(200)
		.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	if user_login.token_expiry < get_current_time() {
		// Token has expired
		Error::as_result()
			.status(200)
			.body(error!(EXPIRED).to_string())?;
	}

	Ok(user_login)
}

pub async fn generate_access_token(
	connection: &mut Transaction<'_, MySql>,
	config: &Settings,
	user_login: &UserLogin,
) -> Result<String, Error> {
	// get roles and permissions of user for rbac here
	// use that info to populate the data in the token_data
	let iat = get_current_time();
	let exp = iat + service::get_access_token_expiry(); // 3 days
	let orgs = db::get_all_organisation_roles_for_user(
		connection,
		&user_login.user_id,
	)
	.await?;

	let User {
		username,
		first_name,
		last_name,
		created,
		id,
		..
	} = db::get_user_by_user_id(connection, &user_login.user_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let user = ExposedUserData {
		id,
		username,
		first_name,
		last_name,
		created,
	};

	let token_data = AccessTokenData::new(iat, exp, orgs, user);
	let jwt = token_data.to_string(config.jwt_secret.as_str())?;

	db::set_refresh_token_expiry(connection, &user_login.login_id, iat, exp)
		.await?;

	Ok(jwt)
}

// function to reset password
// TODO: Remove otp from response
pub async fn forgot_password(
	connection: &mut Transaction<'_, MySql>,
	config: Settings,
	user_id: &str,
) -> Result<(String, String), Error> {
	let user = db::get_user_by_username_or_email(connection, &user_id)
		.await?
		.status(200)
		.body(error!(USER_NOT_FOUND).to_string())?;

	let otp = service::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry = get_current_time() + (1000 * 60 * 60 * 2); // 2 hours

	let token_hash = service::hash(otp.as_bytes())?;

	db::add_password_reset_request(
		connection,
		&user.id,
		&token_hash,
		token_expiry,
	)
	.await?;

	let backup_email =
		db::get_backup_email_for_user(connection, &user.id).await?;

	let mut email_address = String::new();

	if let Some((email_local, PersonalDomain { id: _, name })) = backup_email {
		email_address = format!("{}@{}", email_local.clone(), name.clone());
		task::spawn_blocking(move || {
			mailer::send_password_changed_notification_mail(
				config,
				format!("{}@{}", email_local, name),
			);
		});
	}

	Ok((otp, email_address))
}

pub async fn reset_password(
	connection: &mut Transaction<'_, MySql>,
	new_password: &str,
	token: &str,
	user_id: &[u8],
) -> Result<(), Error> {
	let reset_request =
		db::get_password_reset_request_for_user(connection, &user_id).await?;

	if reset_request.is_none() {
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;
	}
	let reset_request = reset_request.unwrap();

	let success = service::validate_hash(token, &reset_request.token)?;

	if !success {
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;
	}

	let new_password = service::hash(new_password.as_bytes())?;

	db::update_user_password(connection, &user_id, &new_password).await?;

	db::delete_password_reset_request_for_user(connection, &user_id).await?;

	Ok(())
}

pub async fn join_user(
	connection: &mut Transaction<'_, MySql>,
	config: &Settings,
	otp: &str,
	username: &str,
) -> Result<(String, Uuid, String, Option<String>), Error> {
	let user_data = db::get_user_to_sign_up_by_username(connection, &username)
		.await?
		.status(200)
		.body(error!(INVALID_OTP).to_string())?;

	let success = service::validate_hash(otp, &user_data.otp_hash)?;

	if !success {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_OTP).to_string())?;
	}

	if user_data.otp_expiry < get_current_time() {
		Error::as_result()
			.status(200)
			.body(error!(OTP_EXPIRED).to_string())?;
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

	let user_uuid = db::generate_new_user_id(connection).await?;
	let user_id = user_uuid.as_bytes();
	let created = get_current_time();

	if rbac::GOD_USER_ID.get().is_none() {
		rbac::GOD_USER_ID
			.set(user_uuid)
			.expect("GOD_USER_ID was already set");
	}

	let email;
	let welcome_email_to;
	let backup_email_to;
	let email_domain_id: Vec<u8> = Vec::new();
	match user_data.email {
		UserEmailAddressSignUp::Personal(email_address) => {
			// i forgot the alternative for this so i had to split the
			// email_address
			let mut email_address_split = email_address.split('@');

			let email_local = email_address_split
				.next()
				.status(400)
				.body(error!(INVALID_EMAIL).to_string())?;

			let email_domain = email_address_split
				.next()
				.status(400)
				.body(error!(INVALID_DOMAIN_NAME).to_string())?;

			let domain_info =
				db::get_personal_domain_by_name(connection, email_domain)
					.await?;

			let domain_id;

			if domain_info.is_none() {
				domain_id = service::get_or_create_personal_domain_id(
					connection,
					&email_domain,
				)
				.await?
			} else {
				let domain_info = domain_info.unwrap();
				domain_id = domain_info.id;
			}

			email = UserEmailAddress {
				email_local: email_local.to_string(),
				domain_id,
			};
			backup_email_to = None;
			welcome_email_to = email_address;
		}
		UserEmailAddressSignUp::Organisation {
			domain_name,
			email_local,
			backup_email,
			organisation_name,
		} => {
			let organisation_id = service::create_organisation(
				connection,
				&organisation_name,
				user_id,
			)
			.await?;
			let organisation_id = organisation_id.as_bytes();

			let domain_id = service::add_domain_to_organisation(
				connection,
				&domain_name,
				organisation_id,
			)
			.await?
			.as_bytes()
			.to_vec();

			welcome_email_to = format!("{}@{}", email_local, domain_name);

			email = UserEmailAddress {
				email_local: email_local.to_string(),
				domain_id,
			};
			backup_email_to = Some(backup_email);
		}
	}

	db::add_orphaned_personal_email_for_user(connection, &email).await?;
	db::create_user(
		connection,
		user_id,
		&user_data.username,
		&user_data.password,
		&email.email_local,
		&email.domain_id,
		// Add phone number country code etc
		None,
		None,
		(&user_data.first_name, &user_data.last_name),
		created,
	)
	.await?;

	db::set_user_id_for_email(connection, user_id, &email_domain_id).await?;
	// add personal organisation
	let personal_organisation_name = service::get_personal_org_name(username);
	service::create_organisation(
		connection,
		&personal_organisation_name,
		user_id,
	)
	.await?;

	db::delete_user_to_be_signed_up(connection, &user_data.username).await?;

	let user = db::get_user_by_user_id(connection, user_id)
		.await?
		.status(200)
		.body(error!(USER_NOT_FOUND).to_string())?;

	let (jwt, refresh_token) = sign_in_user(connection, user, &config).await?;

	Ok((jwt, refresh_token, welcome_email_to, backup_email_to))
}
