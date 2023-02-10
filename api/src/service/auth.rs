use std::net::IpAddr;

use api_models::{
	models::{
		auth::{PreferredRecoveryOption, RecoveryMethod, SignUpAccountType},
		workspace::billing::{PaymentStatus, TransactionType},
	},
	utils::{DateTime, ResourceType, Uuid},
};
use chrono::{Datelike, Duration, Utc};
use eve_rs::AsError;
use reqwest::Client;

use super::get_ip_address_info;
/// This module validates user info and performs tasks related to user
/// authentication The flow of this file will be:
/// 1. An endpoint will be called from routes layer and the arguments will
/// be supplied to the functions in this file, then the functions might
/// connect with db and return what was required for the endpoint
use crate::{
	db::{self, User, UserLoginType, UserToSignUp, UserWebLogin},
	error,
	models::{rbac, AccessTokenData, ExposedUserData, IpQualityScore},
	service,
	utils::{settings::Settings, validator, Error},
	Database,
};

pub struct JoinUser {
	pub jwt: String,
	pub login_id: Uuid,
	pub refresh_token: Uuid,
	pub welcome_email_to: Option<String>,
	pub recovery_email_to: Option<String>,
	pub recovery_phone_number_to: Option<String>,
}

/// # Description
/// This function is used to check if the username already exists
/// and is according to the criteria for the username
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `username` - A string which contains username to be validated
///
/// # Return
/// This function returns a Result<bool, Error> which contains either a bool
/// which says if the username is valid or an error
///
/// [`Transaction`]: Transaction
pub async fn is_username_allowed(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
) -> Result<bool, Error> {
	if !validator::is_username_valid(username) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_USERNAME).to_string())?;
	}

	let user = db::get_user_by_username(connection, username).await?;
	if user.is_some() {
		return Ok(false);
	}

	// check if user is registered for signup
	let sign_up_status =
		db::get_user_to_sign_up_by_username(connection, username).await?;

	if let Some(status) = sign_up_status {
		// return in-valid (`false`) if expiry is greater than current time
		if status.otp_expiry > Utc::now() {
			return Ok(false);
		}
	}
	Ok(true)
}

/// # Description
/// This function is used to check if the email already exists and
/// is according to the criteria for the email
///
/// # Arguments
/// * `connection` - database save point, more details here:
/// [`Transaction`]
/// * `email` - A string which contains username to be validated
///
/// # Return
/// This function returns a Result<bool, Error> which contains either a bool
/// which says if the email is valid or an error
///
/// [`Transaction`]: Transaction
pub async fn is_email_allowed(
	connection: &mut <Database as sqlx::Database>::Connection,
	email: &str,
) -> Result<bool, Error> {
	if !validator::is_email_valid(email) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_EMAIL).to_string())?;
	}

	let user = db::get_user_by_email(connection, email).await?;
	if user.is_some() {
		return Ok(false);
	}
	// check if the email has already been registered for verifying
	let verify_status =
		db::get_personal_email_to_be_verified_by_email(connection, email)
			.await?;
	if let Some(verify_status) = verify_status {
		if verify_status.verification_token_expiry > Utc::now() {
			return Ok(false);
		}
	}

	let sign_up_status =
		db::get_user_to_sign_up_by_email(connection, email).await?;
	if let Some(status) = sign_up_status {
		if status.otp_expiry > Utc::now() {
			return Ok(false);
		}
	}
	Ok(true)
}
/// # Description
/// This function is used to check if the phone number
/// already exists and is valid
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `phone_country_code` - A string which contains phone number country code
/// * `phone_number` - A string which contains phone_number to be validated
///
/// # Return
///
/// This function returns a Result<bool, Error> which contains either a bool
/// which says if the phone number is valid or an error
///
/// [`Transaction`]: Transaction
pub async fn is_phone_number_allowed(
	connection: &mut <Database as sqlx::Database>::Connection,
	phone_country_code: &str,
	phone_number: &str,
) -> Result<bool, Error> {
	if !validator::is_phone_number_valid(phone_number) {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_PHONE_NUMBER).to_string())?;
	}

	let country_code =
		db::get_phone_country_by_country_code(connection, phone_country_code)
			.await?
			.status(400)
			.body(error!(INVALID_COUNTRY_CODE).to_string())?;

	let user = db::get_user_by_phone_number(
		connection,
		&country_code.country_code,
		phone_number,
	)
	.await?;

	if user.is_some() {
		return Ok(false);
	}

	// check if the email has already been registered for verifying
	let verify_status = db::get_phone_number_to_be_verified_by_phone_number(
		connection,
		&country_code.country_code,
		phone_number,
	)
	.await?;

	if let Some(verify_status) = verify_status {
		if verify_status.verification_token_expiry > Utc::now() {
			return Ok(false);
		}
	}

	let sign_up_status = db::get_user_to_sign_up_by_phone_number(
		connection,
		&country_code.country_code,
		phone_number,
	)
	.await?;

	if let Some(status) = sign_up_status {
		if status.otp_expiry > Utc::now() {
			return Ok(false);
		}
	}
	Ok(true)
}

/// # Description
/// This function is used to create a new user to be signed up and returns an
/// OTP, this function will validate details given by the user, then a resource
/// will be generated for the user according to the type of the account
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `username` - A string which contains username
/// * `account_type` - An enum object which contains the type of resource
///   {Personal, Business}
/// * `password` - A string which contains password of the user
/// * `first_name` - A string which contains first name of the user
/// * `last_name` - A string which contains last name of the user
/// * `recovery_email` - A string which contains recovery email of the user
/// * `recovery_phone_country_code` - A string which contains phone number
///   country code
/// * `recovery_phone_number` - A string which contains phone number of of user
/// * `business_email_local` - A string which contains a pre-existing
///   email_local of the user's
/// business email
/// * `business_domain_name` - A string which contains domain name of the user's
///   business email id
/// * `business_name` - A string which contains user's business name.
///
/// # Return
/// This function returns a `Result<string, error>` which contains either
/// one-time-password to confirm user's email id or phone number and hence
/// complete the registration or an error
///
/// [`Transaction`]: Transaction
pub async fn create_user_join_request(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
	password: &str,
	first_name: &str,
	last_name: &str,
	account_type: &SignUpAccountType,
	recovery_method: &RecoveryMethod,
	coupon_code: Option<&str>,
) -> Result<(UserToSignUp, String), Error> {
	// Check if the username is allowed
	if !is_username_allowed(connection, username).await? {
		Error::as_result()
			.status(200)
			.body(error!(USERNAME_TAKEN).to_string())?;
	}

	// Check if the password passes standards
	if !validator::is_password_valid(password) {
		Error::as_result()
			.status(200)
			.body(error!(PASSWORD_TOO_WEAK).to_string())?;
	}
	// If recovery email is given, extract the local and domain id from it
	let recovery_email_local;
	let recovery_email_domain_id;
	let phone_country_code;
	let phone_number;

	match recovery_method {
		// If phone is provided
		RecoveryMethod::PhoneNumber {
			recovery_phone_country_code,
			recovery_phone_number,
		} => {
			if !is_phone_number_allowed(
				connection,
				recovery_phone_country_code,
				recovery_phone_number,
			)
			.await?
			{
				Error::as_result()
					.status(400)
					.body(error!(PHONE_NUMBER_TAKEN).to_string())?;
			}
			phone_country_code = Some(recovery_phone_country_code.clone());
			phone_number = Some(recovery_phone_number.clone());
			recovery_email_local = None;
			recovery_email_domain_id = None;
		}
		// If recovery_email is only provided
		RecoveryMethod::Email { recovery_email } => {
			// Check if recovery_email is allowed and valid
			if !is_email_allowed(connection, recovery_email).await? {
				Error::as_result()
					.status(200)
					.body(error!(EMAIL_TAKEN).to_string())?;
			}

			if recovery_email.contains('+') {
				log::trace!(
				"Invalid email address: {}, '+' not allowed in email address",
				recovery_email
			);

				return Error::as_result()
					.status(400)
					.body(error!(INVALID_EMAIL).to_string())?;
			}

			// extract the email_local and domain name from it
			// split email into 2 parts and get domain_id
			let (email_local, domain_id) =
				service::split_email_with_domain_id(connection, recovery_email)
					.await?;

			phone_country_code = None;
			phone_number = None;
			recovery_email_local = Some(email_local);
			recovery_email_domain_id = Some(domain_id);
		}
	}

	let otp = service::generate_new_otp();
	let token_expiry = Utc::now() + service::get_join_token_expiry();

	let password = service::hash(password.as_bytes())?;
	let token_hash = service::hash(otp.as_bytes())?;

	let response: UserToSignUp = match account_type {
		SignUpAccountType::Business {
			account_type: _,
			workspace_name,
			business_email_local,
			domain,
		} => {
			let (domain_name, tld) = super::split_domain_and_tld(domain)
				.await
				.status(400)
				.body(error!(INVALID_DOMAIN_NAME).to_string())?;
			let (domain_name, tld) = (domain_name.as_str(), tld.as_str());

			if !validator::is_workspace_name_valid(workspace_name) {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_WORKSPACE_NAME).to_string())?;
			}

			if db::get_workspace_by_name(connection, workspace_name)
				.await?
				.is_some()
			{
				Error::as_result()
					.status(200)
					.body(error!(WORKSPACE_EXISTS).to_string())?;
			}

			let user_sign_up = db::get_user_to_sign_up_by_business_name(
				connection,
				workspace_name,
			)
			.await?;
			if let Some(user_sign_up) = user_sign_up {
				if user_sign_up.otp_expiry < Utc::now() {
					Error::as_result()
						.status(200)
						.body(error!(WORKSPACE_EXISTS).to_string())?;
				}
			}

			if !validator::is_email_valid(&format!(
				"{}@{}",
				business_email_local, domain
			)) {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_EMAIL).to_string())?;
			}

			// Check if there's already a user to sign up with that domain name
			let is_domain_used_for_sign_up =
				service::is_domain_used_for_sign_up(connection, domain).await?;
			if is_domain_used_for_sign_up {
				Error::as_result()
					.status(400)
					.body(error!(DOMAIN_EXISTS).to_string())?;
			}

			db::set_business_user_to_be_signed_up(
				connection,
				username,
				&password,
				(first_name, last_name),
				recovery_email_local.as_deref(),
				recovery_email_domain_id.as_ref(),
				phone_country_code.as_deref(),
				phone_number.as_deref(),
				business_email_local,
				domain_name,
				tld,
				workspace_name,
				&token_hash,
				&token_expiry,
				coupon_code,
			)
			.await?;

			UserToSignUp {
				username: username.to_string(),
				account_type: ResourceType::Business,
				password,
				first_name: first_name.to_string(),
				last_name: last_name.to_string(),
				recovery_email_local,
				recovery_email_domain_id,
				recovery_phone_country_code: phone_country_code,
				recovery_phone_number: phone_number,
				business_email_local: Some(business_email_local.to_string()),
				business_domain_name: Some(format!("{}.{}", domain_name, tld)),
				business_name: Some(workspace_name.to_string()),
				otp_hash: token_hash,
				otp_expiry: token_expiry,
				coupon_code: coupon_code.map(|code| code.to_string()),
			}
		}
		SignUpAccountType::Personal { account_type: _ } => {
			db::set_personal_user_to_be_signed_up(
				connection,
				username,
				&password,
				(first_name, last_name),
				recovery_email_local.as_deref(),
				recovery_email_domain_id.as_ref(),
				phone_country_code.as_deref(),
				phone_number.as_deref(),
				&token_hash,
				&token_expiry,
				coupon_code,
			)
			.await?;

			UserToSignUp {
				username: username.to_string(),
				account_type: ResourceType::Business,
				password,
				first_name: first_name.to_string(),
				last_name: last_name.to_string(),
				recovery_email_local,
				recovery_email_domain_id,
				recovery_phone_country_code: phone_country_code,
				recovery_phone_number: phone_number,
				business_email_local: None,
				business_domain_name: None,
				business_name: None,
				otp_hash: token_hash,
				otp_expiry: token_expiry,
				coupon_code: coupon_code.map(|code| code.to_string()),
			}
		}
	};

	Ok((response, otp))
}

/// # Description
/// This function is used to create a record when a user logs into the system,
/// this record contains six parameters:
/// 1. login_id
/// login_id is used to give to a unique identity to the current logged in user
/// 2. user_id
/// user_id is the identity of the user currently logged in
/// 3. last_activity
/// last_activity is the most recent task the user has performed on the api,
/// when the user logs in, the last activity is set to the time of login
/// 4. last_login
/// last_login used to show the last time user was logged in. When the user logs
/// in, last_login updates with the time of log in
/// 5. refresh_token
/// refresh_token is used to generate access token, and access token is for
/// authenticating the user for the current session
/// 6. token_expiry
/// token_expiry is used to set the expiry time for newly generated refresh
/// token
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user_id` - an unsigned 8 bit integer array which represents the id of the
///   user
///
/// # Returns
/// This function returns a `Result<(UserLogin, Uuid), error>` which contains an
/// instance of UserLogin with a refresh token or an error
///
/// [`UserLogin`]: UserLogin
pub async fn create_login_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	created_ip: &IpAddr,
	user_agent: &str,
	config: &Settings,
) -> Result<(UserWebLogin, Uuid), Error> {
	let login_id = db::generate_new_login_id(connection).await?;
	let (refresh_token, hashed_refresh_token) =
		service::generate_new_refresh_token_for_user(connection, user_id)
			.await?;
	let now = Utc::now();
	let expiry = now + service::get_refresh_token_expiry();
	let ipinfo = get_ip_address_info(created_ip, &config.ipinfo_token).await?;
	let (lat, lng) = ipinfo.loc.split_once(',').status(500)?;
	let (lat, lng): (f64, f64) = (lat.parse()?, lng.parse()?);

	db::add_new_user_login(
		connection,
		&login_id,
		user_id,
		&UserLoginType::WebLogin,
		&now,
	)
	.await?;

	db::add_new_web_login(
		connection,
		&login_id,
		user_id,
		&hashed_refresh_token,
		&expiry,
		&now,
		created_ip,
		lat,
		lng,
		&ipinfo.country,
		&ipinfo.region,
		&ipinfo.city,
		&ipinfo.timezone,
		&now,
		&now,
		created_ip,
		lat,
		lng,
		&ipinfo.country,
		&ipinfo.region,
		&ipinfo.city,
		&ipinfo.timezone,
		user_agent,
	)
	.await?;
	let user_login = UserWebLogin {
		login_id,
		user_id: user_id.clone(),
		refresh_token: hashed_refresh_token,
		token_expiry: now + service::get_refresh_token_expiry(),
		created: now,
		created_ip: *created_ip,
		created_location_latitude: lat,
		created_location_longitude: lng,
		created_country: ipinfo.country.clone(),
		created_region: ipinfo.region.clone(),
		created_city: ipinfo.city.clone(),
		created_timezone: ipinfo.timezone.clone(),
		last_activity: now,
		last_login: now,
		last_activity_ip: *created_ip,
		last_activity_location_latitude: lat,
		last_activity_location_longitude: lng,
		last_activity_user_agent: user_agent.to_string(),
		last_activity_country: ipinfo.country,
		last_activity_region: ipinfo.region,
		last_activity_city: ipinfo.city,
		last_activity_timezone: ipinfo.timezone,
	};

	Ok((user_login, refresh_token))
}

/// # Description
/// This function is used to log in a user, it calls [`create_login_for_user()`]
/// to get [`UserLogin`] object using which it generates a new refresh token and
/// then generate an access token through the newly generated refresh token
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user_id` - an unsigned 8 bit integer array which represents the id of the
///   user
/// * `config` - An object of [`Settings`] struct which stores configuration of
///   the whole API
///
/// # Returns
/// This function returns a `Result<(UserLogin, String, Uuid), Error>`
/// containing the user login, jwt and refresh token or an error
///
/// [`create_login_for_user()`]: self.create_login_for_user()
/// [`UserLogin`]: UserLogin
/// [`Transaction`]: Transaction
/// [`Settings]: Settings
pub async fn sign_in_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	created_ip: &IpAddr,
	user_agent: &str,
	config: &Settings,
) -> Result<(UserWebLogin, String, Uuid), Error> {
	let (user_login, refresh_token) = create_login_for_user(
		connection, user_id, created_ip, user_agent, config,
	)
	.await?;

	let jwt = generate_access_token(connection, &user_login, config).await?;

	Ok((user_login, jwt, refresh_token))
}

/// # Description
/// This function is used to get the login details of the user
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `login_id` - an unsigned 8 bit integer array which represents the id of
/// the user
///
/// # Returns
/// This function returns `Result<UserLogin, Error>` containing an instance of
/// UserLogin or an error
///
/// [`UserLogin`]: UserLogin
/// [`Transaction`]: Transaction
pub async fn get_user_login_for_login_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
) -> Result<UserWebLogin, Error> {
	let user_login = db::get_user_web_login(connection, login_id)
		.await?
		.status(401)
		.body(error!(UNAUTHORIZED).to_string())?;

	if user_login.token_expiry < Utc::now() {
		// Token has expired
		Error::as_result()
			.status(200)
			.body(error!(EXPIRED).to_string())?;
	}

	Ok(user_login)
}
/// # Description
/// This function is used to generate access token for the logged in user
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `config` - An object of [`Settings`] struct which stores configuration of
/// the whole API
/// * `user_login` - an object of struct [`UserLogin`]
///
/// # Returns
/// This function returns a `Result<String, Error>` containing a jwt token
///
/// [`Transaction`]: Transaction
pub async fn generate_access_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_login: &UserWebLogin,
	config: &Settings,
) -> Result<String, Error> {
	// get roles and permissions of user for rbac here
	// use that info to populate the data in the token_data
	let now = Utc::now();
	let exp = now + service::get_access_token_expiry(); // 3 days
	let refresh_token_expiry = now + service::get_refresh_token_expiry();
	let permissions = db::get_all_workspace_role_permissions_for_user(
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

	db::set_web_login_expiry(
		connection,
		&user_login.login_id,
		&now,
		&refresh_token_expiry,
	)
	.await?;

	let user = ExposedUserData {
		id,
		username,
		first_name,
		last_name,
		created,
	};

	let token_data = AccessTokenData::new(
		now,
		exp,
		permissions,
		user_login.login_id.clone(),
		user,
	);
	let jwt = token_data.to_string(config.jwt_secret.as_str())?;

	Ok(jwt)
}

/// # Description
/// This function takes care of generating an OTP and sending it
/// to the preferred Recovery option chosen by the user.
/// response will NOT contain the OTP
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `user_id` - an unsigned 8 bit integer array which represents the id of the
///   user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn forgot_password(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &str,
	preferred_recovery_option: &PreferredRecoveryOption,
) -> Result<(), Error> {
	let user =
		db::get_user_by_username_email_or_phone_number(connection, user_id)
			.await?
			.status(200)
			.body(error!(USER_NOT_FOUND).to_string())?;

	let otp = service::generate_new_otp();

	let token_expiry = Utc::now() + Duration::hours(2);

	let token_hash = service::hash(otp.as_bytes())?;

	db::add_password_reset_request(
		connection,
		&user.id,
		&token_hash,
		&token_expiry,
	)
	.await?;

	service::send_forgot_password_otp(
		connection,
		user,
		preferred_recovery_option,
		&otp,
	)
	.await?;

	Ok(())
}

/// # Description
/// This function updates the password of user
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `new_password` - a string containing new password of user
/// * `token` - a string containing a reset request token to verify if the reset
///   password request is
/// valid or not
/// * `user_id` - an unsigned 8 bit integer array which represents the id of the
///   user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Transaction`]: Transaction
pub async fn reset_password(
	connection: &mut <Database as sqlx::Database>::Connection,
	new_password: &str,
	token: &str,
	user_id: &Uuid,
) -> Result<(), Error> {
	let reset_request =
		db::get_password_reset_request_for_user(connection, user_id)
			.await?
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	let user = db::get_user_by_user_id(connection, user_id)
		.await?
		.status(500)
		.body(error!(USER_NOT_FOUND).to_string())?;

	// check password strength
	if !validator::is_password_valid(new_password) {
		Error::as_result()
			.status(200)
			.body(error!(PASSWORD_TOO_WEAK).to_string())?;
	}

	let success = service::validate_hash(token, &reset_request.token)?;

	if !success {
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;
	}

	let is_password_same =
		service::validate_hash(new_password, &user.password)?;

	if is_password_same {
		Error::as_result()
			.status(400)
			.body(error!(PASSWORD_UNCHANGED).to_string())?;
	}

	let new_password = service::hash(new_password.as_bytes())?;

	db::update_user_password(connection, user_id, &new_password).await?;

	db::delete_password_reset_request_for_user(connection, user_id).await?;

	Ok(())
}

/// # Description
/// This function is used to register user in database
/// required parameters for personal account:
///     1. username
///     2. password
///     3. account_type
///     4. first_name
///     5. last_name
///     6. (recovery_email_local, recovery_email_domain_id) OR
///     7. (recovery_phone_country_code, recovery_phone_number)
/// extra parameters required for business account:
///     1. business_domain_name
///     2. business_name
///     3. business_email_local
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `config` - An object of [`Settings`] struct which stores configuration of
///   the whole API
/// * `otp` - A string which contains One-Time-Password
/// * `username` - A string containing username of the user
/// # Returns
/// This function returns `Result<JoinUser, Error>` containing a struct called
/// [`JoinUser`]
///
/// [`Transaction`]: Transaction
/// ['JoinUser`]: JoinUser
pub async fn join_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	otp: &str,
	username: &str,
	created_ip: &IpAddr,
	user_agent: &str,
	config: &Settings,
) -> Result<JoinUser, Error> {
	let user_data = db::get_user_to_sign_up_by_username(connection, username)
		.await?
		.status(200)
		.body(error!(OTP_EXPIRED).to_string())?;

	let success = service::validate_hash(otp, &user_data.otp_hash)?;

	if !success {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_OTP).to_string())?;
	}

	if user_data.otp_expiry < Utc::now() {
		Error::as_result()
			.status(200)
			.body(error!(OTP_EXPIRED).to_string())?;
	}

	// First create user,
	// Then create personal workspace,
	// Then set email to recovery email if personal account,
	// And finally send the token, along with the email to the user

	let user_id = db::generate_new_user_id(connection).await?;
	let now = Utc::now();

	if rbac::GOD_USER_ID.get().is_none() {
		rbac::GOD_USER_ID
			.set(user_id.clone())
			.expect("GOD_USER_ID was already set");
	}

	db::begin_deferred_constraints(connection).await?;

	if let Some((email_local, domain_id)) = user_data
		.recovery_email_local
		.as_ref()
		.zip(user_data.recovery_email_domain_id.as_ref())
	{
		db::add_personal_email_for_user(
			connection,
			&user_id,
			email_local,
			domain_id,
		)
		.await?;
	} else if let Some((phone_country_code, phone_number)) = user_data
		.recovery_phone_country_code
		.as_ref()
		.zip(user_data.recovery_phone_number.as_ref())
	{
		db::add_phone_number_for_user(
			connection,
			&user_id,
			phone_country_code,
			phone_number,
		)
		.await?;
	} else {
		log::error!(
            "Got neither recovery email, nor recovery phone number while signing up user: {}",
            user_data.username
        );

		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	db::create_user(
		connection,
		&user_id,
		&user_data.username,
		&user_data.password,
		(&user_data.first_name, &user_data.last_name),
		&now,
		user_data.recovery_email_local.as_deref(),
		user_data.recovery_email_domain_id.as_ref(),
		user_data.recovery_phone_country_code.as_deref(),
		user_data.recovery_phone_number.as_deref(),
		3,
		user_data.coupon_code.as_deref(),
	)
	.await?;

	db::end_deferred_constraints(connection).await?;

	let recovery_email: Vec<String> = if let Some((email_local, domain_id)) =
		user_data
			.recovery_email_local
			.as_deref()
			.zip(user_data.recovery_email_domain_id.as_ref())
	{
		let domain = db::get_personal_domain_by_id(connection, domain_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

		vec![format!("{}@{}", email_local, domain.name)]
	} else {
		vec![]
	};

	// add personal workspace
	let personal_workspace_name =
		service::get_personal_workspace_name(&user_id);
	let personal_workspace_id = service::create_workspace(
		connection,
		&personal_workspace_name,
		&user_id,
		true,
		&recovery_email,
		config,
	)
	.await?;

	//Check and select recovery options as provided
	let welcome_email_to; // Send the "welcome to patr" email here
	let recovery_email_to; // Send "this email is a recovery email for ..." here
	let recovery_phone_number_to; // Notify this phone that it's a recovery phone number

	if let Some((email_local, domain_id)) = user_data
		.recovery_email_local
		.as_ref()
		.zip(user_data.recovery_email_domain_id.as_ref())
	{
		let email_domain = db::get_personal_domain_by_id(connection, domain_id)
			.await?
			.status(500)?
			.name;
		welcome_email_to = Some(format!("{}@{}", email_local, email_domain));
		recovery_email_to = Some(format!("{}@{}", email_local, email_domain));
		recovery_phone_number_to = None;
	} else if let Some((phone_country_code, phone_number)) = user_data
		.recovery_phone_country_code
		.as_ref()
		.zip(user_data.recovery_phone_number.as_ref())
	{
		let country = db::get_phone_country_by_country_code(
			connection,
			phone_country_code,
		)
		.await?
		.status(500)?;
		recovery_phone_number_to =
			Some(format!("+{}{}", country.phone_code, phone_number));
		welcome_email_to = None;
		recovery_email_to = None;
	} else {
		log::error!(
                "Got neither recovery email, nor recovery phone number while signing up user: {}",
                user_data.username
            );
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	if user_data.account_type.is_personal() {
		if let Some(coupon_code) = user_data.coupon_code.as_deref() {
			if let Some(coupon_detail) =
				db::get_sign_up_coupon_by_code(connection, coupon_code).await?
			{
				// Check coupon validity condition
				let is_coupon_valid = coupon_detail
					.expiry
					.map(|expiry| expiry > now)
					.unwrap_or(true) && coupon_detail
					.uses_remaining
					.map(|uses_remaining| uses_remaining > 0)
					.unwrap_or(true) && coupon_detail
					.credits_in_cents > 0;

				// It's not expired, it has usage remaining, AND it has a
				// non zero positive credit value. Give them some fucking
				// credits
				if is_coupon_valid {
					let transaction_id =
						db::generate_new_transaction_id(connection).await?;
					db::create_transaction(
						connection,
						&personal_workspace_id,
						&transaction_id,
						now.month() as i32,
						coupon_detail.credits_in_cents,
						Some("coupon-credits"),
						&DateTime::from(now),
						&TransactionType::Credits,
						&PaymentStatus::Success,
						Some("Coupon credits"),
					)
					.await?;
					if let Some(uses_remaining) = coupon_detail.uses_remaining {
						db::update_coupon_code_uses_remaining(
							connection,
							coupon_code,
							uses_remaining.saturating_sub(1),
						)
						.await?;
					}
				}
			}
		}
	}

	if let Some(ref recovery_email) = welcome_email_to {
		// Reference for APIs docs of ipqualityscore can be found in this
		// url https://www.ipqualityscore.com/documentation/email-validation/overview
		let email_spam_result = Client::new()
			.get(format!(
				"{}/{}/{}",
				config.ip_quality.host, config.ip_quality.token, recovery_email
			))
			.send()
			.await?
			.json::<IpQualityScore>()
			.await?;

		let disposable = email_spam_result.disposable;
		let is_spam = email_spam_result.fraud_score > 75;

		let workspaces =
			db::get_all_workspaces_for_user(connection, &user_id).await?;

		if disposable || is_spam {
			for workspace in &workspaces {
				if disposable {
					db::set_resource_limit_for_workspace(
						connection,
						&workspace.id,
						0,
						0,
						0,
						0,
						0,
						0,
						0,
					)
					.await?;
				} else {
					db::mark_workspace_as_spam(connection, &workspace.id)
						.await?;
				}
			}
		}
	}

	db::delete_user_to_be_signed_up(connection, &user_data.username).await?;

	let (UserWebLogin { login_id, .. }, jwt, refresh_token) =
		sign_in_user(connection, &user_id, created_ip, user_agent, config)
			.await?;
	let response = JoinUser {
		jwt,
		login_id,
		refresh_token,
		welcome_email_to,
		recovery_email_to,
		recovery_phone_number_to,
	};

	Ok(response)
}

pub async fn resend_user_sign_up_otp(
	connection: &mut <Database as sqlx::Database>::Connection,
	username: &str,
	password: &str,
) -> Result<(UserToSignUp, String), Error> {
	let user_to_sign_up =
		db::get_user_to_sign_up_by_username(connection, username)
			.await?
			.status(400)
			.body(error!(USER_NOT_FOUND).to_string())?;

	let success = service::validate_hash(password, &user_to_sign_up.password)?;

	if !success {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_PASSWORD).to_string())?;
	}

	let otp = service::generate_new_otp();
	let token_expiry = Utc::now() + service::get_join_token_expiry();

	let token_hash = service::hash(otp.as_bytes())?;

	db::update_user_to_sign_up_with_otp(
		connection,
		username,
		&token_hash,
		&token_expiry,
	)
	.await?;

	Ok((
		db::get_user_to_sign_up_by_username(connection, username)
			.await?
			.status(400)
			.body(error!(USER_NOT_FOUND).to_string())?,
		otp,
	))
}
