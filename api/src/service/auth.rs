use eve_rs::AsError;
use serde_json::Value;
use sqlx::{MySql, Transaction};
use tokio::task;
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
		constants::AccountType,
		get_current_time,
		mailer,
		settings::Settings,
		validator,
		AsErrorData,
		EveError as Error,
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

	let password =
		service::hash(password.as_bytes(), config.password_salt.as_bytes())?;
	let token_hash =
		service::hash(otp.as_bytes(), config.password_salt.as_bytes())?;

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

/// function to sign in a user
/// Returns: JWT (String), Refresh Token (Uuid)
pub async fn sign_in_user(
	connection: &mut Transaction<'_, MySql>,
	user: User,
	config: &Settings,
) -> Result<(String, Uuid), Error> {
	// generate JWT
	let iat = get_current_time();
	let exp = iat + (1000 * 3600 * 24 * 3); // 3 days
	let orgs =
		db::get_all_organisation_roles_for_user(connection, &user.id).await?;

	let user = ExposedUserData {
		id: user.id,
		username: user.username,
		first_name: user.first_name,
		last_name: user.last_name,
		created: user.created,
	};

	let refresh_token = Uuid::new_v4();

	db::add_user_login(
		connection,
		refresh_token.as_bytes(),
		iat + (1000 * 60 * 60 * 24 * 30), // 30 days
		&user.id,
		iat,
		iat,
	)
	.await?;

	let token_data = AccessTokenData::new(iat, exp, orgs, user);
	let jwt = token_data.to_string(config.jwt_secret.as_str())?;

	Ok((jwt, refresh_token))
}

pub async fn get_access_token_data(
	connection: &mut Transaction<'_, MySql>,
	config: Settings,
	refresh_token: &str,
) -> Result<String, Error> {
	let refresh_token = if let Ok(uuid) = Uuid::parse_str(&refresh_token) {
		uuid
	} else {
		return Err(error!(WRONG_PARAMETERS));
	};
	let refresh_token = refresh_token.as_bytes();

	let user_login = db::get_user_login(connection, refresh_token).await?;

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
		connection,
		&user_login.user_id,
	)
	.await?;

	let user_id = user_login.user_id;
	let user_data = db::get_user_by_user_id(connection, &user_id)
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
	let jwt = token_data.to_string(config.jwt_secret.as_str())?;

	db::set_refresh_token_expiry(connection, refresh_token, iat, exp).await?;

	Ok(jwt)
}

// function to reset password
// TODO: Remove otp from response
pub async fn forgot_password(
	connection: &mut Transaction<'_, MySql>,
	config: Settings,
	user_id: &str,
) -> Result<String, Value> {
	let user = db::get_user_by_username_or_email(connection, &user_id)
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
		connection,
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
	connection: &mut Transaction<'_, MySql>,
	config: &Settings,
	// pool: Pool<MySql>,
	new_password: &str,
	token: &str,
	user_id: &[u8],
) -> Result<(), Value> {
	let reset_request =
		db::get_password_reset_request_for_user(connection, &user_id)
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

	db::update_user_password(connection, &user_id, &new_password)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	db::delete_password_reset_request_for_user(connection, &user_id)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

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

	let success = service::validate_hash(
		otp.as_bytes(),
		config.password_salt.as_bytes(),
		&user_data.otp_hash,
	)?;

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

	let user_uuid = Uuid::new_v4();
	let user_id = user_uuid.as_bytes();
	let created = get_current_time();

	if rbac::GOD_USER_ID.get().is_none() {
		rbac::GOD_USER_ID
			.set(user_uuid)
			.expect("GOD_USER_ID was already set");
	}

	db::create_user(
		connection,
		user_id,
		&user_data.username,
		&user_data.password,
		&user_data.backup_email,
		(&user_data.first_name, &user_data.last_name),
		created,
	)
	.await?;

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
			let organisation_id = db::generate_new_resource_id(connection)
				.await
				.map_err(|_| error!(SERVER_ERROR))?;
			let organisation_id = organisation_id.as_bytes();
			db::create_orphaned_resource(
				connection,
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
				connection,
				organisation_id,
				&organisation_name,
				user_id,
				get_current_time(),
			)
			.await
			.map_err(|_| error!(SERVER_ERROR))?;
			db::set_resource_owner_id(
				connection,
				organisation_id,
				organisation_id,
			)
			.await
			.map_err(|_| error!(SERVER_ERROR))?;

			let domain_id = db::generate_new_resource_id(connection)
				.await
				.map_err(|_| error!(SERVER_ERROR))?;
			let domain_id = domain_id.as_bytes().to_vec();

			db::create_resource(
				connection,
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
				connection,
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
	let organisation_id = db::generate_new_resource_id(connection)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;
	let organisation_id = organisation_id.as_bytes();
	let organisation_name =
		format!("personal-organisation-{}", hex::encode(user_id));

	db::create_orphaned_resource(
		connection,
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
		connection,
		organisation_id,
		&organisation_name,
		user_id,
		get_current_time(),
	)
	.await
	.map_err(|_| error!(SERVER_ERROR))?;
	db::set_resource_owner_id(connection, organisation_id, organisation_id)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	db::add_email_for_user(connection, user_id, email)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;
	db::delete_user_to_be_signed_up(connection, &user_data.username)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	let user = if let Some(user) =
		db::get_user_by_username_or_email(connection, &user_data.username)
			.await
			.map_err(|_| error!(SERVER_ERROR))?
	{
		user
	} else {
		return Err(error!(USER_NOT_FOUND));
	};

	let (jwt, refresh_token) = sign_in_user(connection, user, &config)
		.await
		.map_err(|_| error!(SERVER_ERROR))?;

	Ok((
		jwt,
		refresh_token,
		welcome_email_to,
		backup_email_notification_to,
	))
}

pub async fn create_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_name: &str,
	super_admin_id: &[u8],
) -> Result<Uuid, Error> {
	todo!("create organisation and return org id")
}
