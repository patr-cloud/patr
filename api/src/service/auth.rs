use eve_rs::AsError;
use uuid::Uuid;

/// This module validates user info and performs tasks related to user
/// authentication The flow of this file will be:
/// 1. An endpoint will be called from routes layer and the arguments will
/// be supplied to the functions in this file, then the functions might
/// connect with db and return what was required for the endpoint
use crate::{
	db,
	error,
	models::{
		db_mapping::{
			JoinUser,
			PreferredRecoveryOption,
			User,
			UserLogin,
			UserToSignUp,
		},
		rbac,
		AccessTokenData,
		ExposedUserData,
	},
	service::{self, get_refresh_token_expiry},
	utils::{
		constants::ResourceOwnerType,
		get_current_time_millis,
		settings::Settings,
		validator,
		Error,
	},
	Database,
};

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
	if !validator::is_username_valid(&username) {
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
		if status.otp_expiry > get_current_time_millis() {
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
	if !validator::is_email_valid(&email) {
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
		if verify_status.verification_token_expiry > get_current_time_millis() {
			return Ok(false);
		}
	}

	let sign_up_status =
		db::get_user_to_sign_up_by_email(connection, email).await?;
	if let Some(status) = sign_up_status {
		if status.otp_expiry > get_current_time_millis() {
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
		if verify_status.verification_token_expiry > get_current_time_millis() {
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
		if status.otp_expiry > get_current_time_millis() {
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
///   {Personal, Organisation}
/// * `password` - A string which contains password of the user
/// * `first_name` - A string which contains first name of the user
/// * `last_name` - A string which contains last name of the user
/// * `backup_email` - A string which contains recovery email of the user
/// * `backup_phone_country_code` - A string which contains phone number country
///   code
/// * `backup_phone_number` - A string which contains phone number of of user
/// * `org_email_local` - A string which contains a pre-existing email_local of
///   the user's
/// organisation email
/// * `org_domain_name` - A string which contains domain name of the user's
///   organisation's email id
/// * `organisation_name` - A string which contains user's organisation name.
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
	account_type: ResourceOwnerType,
	password: &str,
	(first_name, last_name): (&str, &str),

	backup_email: Option<&str>,
	backup_phone_country_code: Option<&str>,
	backup_phone_number: Option<&str>,

	org_email_local: Option<&str>,
	org_domain_name: Option<&str>,
	organisation_name: Option<&str>,
) -> Result<(UserToSignUp, String), Error> {
	// Check if the username is allowed
	if !is_username_allowed(connection, username).await? {
		Error::as_result()
			.status(200)
			.body(error!(USERNAME_TAKEN).to_string())?;
	}

	// Check if the password passes standards
	if !validator::is_password_valid(&password) {
		Error::as_result()
			.status(200)
			.body(error!(PASSWORD_TOO_WEAK).to_string())?;
	}

	let response: UserToSignUp;
	// If backup email is given, extract the local and domain id from it
	let backup_email_local;
	let backup_email_domain_id;
	let phone_country_code;
	let phone_number;

	match (backup_email, backup_phone_country_code, backup_phone_number) {
		// If phone is provided
		(None, Some(country_code), Some(number)) => {
			if !is_phone_number_allowed(connection, country_code, number)
				.await?
			{
				Error::as_result()
					.status(400)
					.body(error!(PHONE_NUMBER_TAKEN).to_string())?;
			}
			phone_country_code = Some(country_code);
			phone_number = Some(number);
			backup_email_local = None;
			backup_email_domain_id = None;
		}
		// If backup_email is only provided
		(Some(backup_email), None, None) => {
			// Check if backup_email is allowed and valid
			if !is_email_allowed(connection, backup_email).await? {
				Error::as_result()
					.status(200)
					.body(error!(EMAIL_TAKEN).to_string())?;
			}

			// extract the email_local and domain name from it
			// split email into 2 parts and get domain_id
			let (email_local, domain_id) =
				service::split_email_with_domain_id(connection, backup_email)
					.await?;

			phone_country_code = None;
			phone_number = None;
			backup_email_local = Some(email_local);
			backup_email_domain_id = Some(domain_id);
		}
		// If both or neither recovery options are provided
		_ => {
			return Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()));
		}
	}
	let backup_email_domain_id = backup_email_domain_id.as_deref();

	let otp = service::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);
	let token_expiry =
		get_current_time_millis() + service::get_join_token_expiry();

	let password = service::hash(password.as_bytes())?;
	let token_hash = service::hash(otp.as_bytes())?;

	match account_type {
		ResourceOwnerType::Organisation => {
			let org_domain_name = org_domain_name
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
			let organisation_name = organisation_name
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
			let org_email_local = org_email_local
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;

			if !validator::is_domain_name_valid(org_domain_name).await {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_DOMAIN_NAME).to_string())?;
			}

			if !validator::is_organisation_name_valid(organisation_name) {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_ORGANISATION_NAME).to_string())?;
			}

			if db::get_organisation_by_name(connection, organisation_name)
				.await?
				.is_some()
			{
				Error::as_result()
					.status(200)
					.body(error!(ORGANISATION_EXISTS).to_string())?;
			}

			let user_sign_up = db::get_user_to_sign_up_by_organisation_name(
				connection,
				&organisation_name,
			)
			.await?;
			if let Some(user_sign_up) = user_sign_up {
				if user_sign_up.otp_expiry < get_current_time_millis() {
					Error::as_result()
						.status(200)
						.body(error!(ORGANISATION_EXISTS).to_string())?;
				}
			}

			if !validator::is_email_valid(&format!(
				"{}@{}",
				org_email_local, org_domain_name
			)) {
				Error::as_result()
					.status(200)
					.body(error!(INVALID_EMAIL).to_string())?;
			}

			db::set_organisation_user_to_be_signed_up(
				connection,
				username,
				&password,
				(first_name, last_name),
				backup_email_local.as_deref(),
				backup_email_domain_id,
				phone_country_code,
				phone_number,
				org_email_local,
				org_domain_name,
				organisation_name,
				&token_hash,
				token_expiry,
			)
			.await?;
			// let check = backup_email_domain_id.map(|s| s.to_vec())

			response = UserToSignUp {
				username: username.to_string(),
				account_type: ResourceOwnerType::Organisation,
				password,
				first_name: first_name.to_string(),
				last_name: last_name.to_string(),
				backup_email_local,
				backup_email_domain_id: backup_email_domain_id
					.map(|s| s.to_vec()),
				backup_phone_country_code: phone_country_code
					.map(|s| s.to_string()),
				backup_phone_number: phone_number.map(|s| s.to_string()),
				org_email_local: Some(org_email_local.to_string()),
				org_domain_name: Some(org_domain_name.to_string()),
				organisation_name: Some(organisation_name.to_string()),
				otp_hash: token_hash,
				otp_expiry: token_expiry,
			}
		}
		ResourceOwnerType::Personal => {
			db::set_personal_user_to_be_signed_up(
				connection,
				username,
				&password,
				(first_name, last_name),
				backup_email_local.as_deref(),
				backup_email_domain_id,
				phone_country_code,
				phone_number,
				&token_hash,
				token_expiry,
			)
			.await?;

			response = UserToSignUp {
				username: username.to_string(),
				account_type: ResourceOwnerType::Organisation,
				password,
				first_name: first_name.to_string(),
				last_name: last_name.to_string(),
				backup_email_local,
				backup_email_domain_id: backup_email_domain_id
					.map(|s| s.to_vec()),
				backup_phone_country_code: phone_country_code
					.map(|s| s.to_string()),
				backup_phone_number: phone_number.map(|s| s.to_string()),
				org_email_local: None,
				org_domain_name: None,
				organisation_name: None,
				otp_hash: token_hash,
				otp_expiry: token_expiry,
			}
		}
	}

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
/// This function returns a `Result<UserLogin, error>` which contains an
/// instance of UserLogin or an error
///
/// [`UserLogin`]: UserLogin
pub async fn create_login_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
) -> Result<UserLogin, Error> {
	let login_id = db::generate_new_login_id(connection).await?;
	let (refresh_token, hashed_refresh_token) =
		service::generate_new_refresh_token_for_user(connection, user_id)
			.await?;
	let iat = get_current_time_millis();

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
/// This function returns a `Result<(String, Uuid, Uuid), Error>` containing
/// jwt, login_id, and refresh token or an error
///
/// [`create_login_for_user()`]: self.create_login_for_user()
/// [`UserLogin`]: UserLogin
/// [`Transaction`]: Transaction
/// [`Settings]: Settings
pub async fn sign_in_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &[u8],
	config: &Settings,
) -> Result<(String, Uuid, Uuid), Error> {
	let refresh_token = Uuid::new_v4();

	let user_login = create_login_for_user(connection, &user_id).await?;

	let jwt = generate_access_token(connection, &config, &user_login).await?;

	Ok((
		jwt,
		Uuid::from_slice(&user_login.login_id).unwrap(),
		refresh_token,
	))
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
	login_id: &[u8],
) -> Result<UserLogin, Error> {
	let user_login = db::get_user_login(connection, login_id)
		.await?
		.status(200)
		.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	if user_login.token_expiry < get_current_time_millis() {
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
	config: &Settings,
	user_login: &UserLogin,
) -> Result<String, Error> {
	// get roles and permissions of user for rbac here
	// use that info to populate the data in the token_data
	let iat = get_current_time_millis();
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

	let token_data = AccessTokenData::new(
		iat,
		exp,
		orgs,
		hex::encode(&user_login.login_id),
		user,
	);
	let jwt = token_data.to_string(config.jwt_secret.as_str())?;

	db::set_login_expiry(connection, &user_login.login_id, iat, exp).await?;

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
	preferred_recovery_option: PreferredRecoveryOption,
) -> Result<(), Error> {
	let user =
		db::get_user_by_username_email_or_phone_number(connection, &user_id)
			.await?
			.status(200)
			.body(error!(USER_NOT_FOUND).to_string())?;

	let otp = service::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry = get_current_time_millis() + (1000 * 60 * 60 * 2); // 2 hours

	let token_hash = service::hash(otp.as_bytes())?;

	db::add_password_reset_request(
		connection,
		&user.id,
		&token_hash,
		token_expiry,
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

/// # Description
/// This function is used to register user in database
/// required parameters for personal account:
///     1. username
///     2. password
///     3. account_type
///     4. first_name
///     5. last_name
///     6. (backup_email_local, backup_email_domain_id) OR
///     7. (backup_phone_country_code, backup_phone_number)
/// extra parameters required for organisation account:
///     1. domain_name
///     2. organisation_name
///     3. backup_email
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
	config: &Settings,
	otp: &str,
	username: &str,
) -> Result<JoinUser, Error> {
	let user_data = db::get_user_to_sign_up_by_username(connection, &username)
		.await?
		.status(200)
		.body(error!(OTP_EXPIRED).to_string())?;

	let success = service::validate_hash(otp, &user_data.otp_hash)?;

	if !success {
		Error::as_result()
			.status(200)
			.body(error!(INVALID_OTP).to_string())?;
	}

	if user_data.otp_expiry < get_current_time_millis() {
		Error::as_result()
			.status(200)
			.body(error!(OTP_EXPIRED).to_string())?;
	}

	// First create user,
	// Then create an organisation if an org account,
	// Then add the domain if org account,
	// Then create personal org regardless,
	// Then set email to backup email if personal account,
	// And finally send the token, along with the email to the user

	let user_uuid = db::generate_new_user_id(connection).await?;
	let user_id = user_uuid.as_bytes();
	let created = get_current_time_millis();

	if rbac::GOD_USER_ID.get().is_none() {
		rbac::GOD_USER_ID
			.set(user_uuid)
			.expect("GOD_USER_ID was already set");
	}

	let backup_email_local = user_data.backup_email_local.as_deref();
	let backup_email_domain_id = user_data.backup_email_domain_id.as_deref();
	let backup_phone_country_code =
		user_data.backup_phone_country_code.as_deref();
	let backup_phone_number = user_data.backup_phone_number.as_deref();
	db::begin_deferred_constraints(connection).await?;

	if let Some((email_local, domain_id)) = user_data
		.backup_email_local
		.as_ref()
		.zip(user_data.backup_email_domain_id.as_ref())
	{
		db::add_personal_email_for_user(
			connection,
			user_id,
			&email_local,
			&domain_id,
		)
		.await?;
	} else if let Some((phone_country_code, phone_number)) = user_data
		.backup_phone_country_code
		.as_ref()
		.zip(user_data.backup_phone_number.as_ref())
	{
		db::add_phone_number_for_user(
			connection,
			user_id,
			&phone_country_code,
			&phone_number,
		)
		.await?;
	} else {
		log::error!(
			"Got neither backup email, nor backup phone number while signing up user: {}",
			user_data.username
		);
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	db::create_user(
		connection,
		user_id,
		&user_data.username,
		&user_data.password,
		(&user_data.first_name, &user_data.last_name),
		created,
		backup_email_local,
		backup_email_domain_id,
		backup_phone_country_code,
		backup_phone_number,
	)
	.await?;
	db::end_deferred_constraints(connection).await?;

	let welcome_email_to; // Send the "welcome to vicara" email here
	let backup_email_to; // Send "this email is a backup email for ..." here
	let backup_phone_number_to; // Notify this phone that it's a backup phone number

	// For an organisation, create the organisation and domain
	if let ResourceOwnerType::Organisation = user_data.account_type {
		let organisation_id = service::create_organisation(
			connection,
			&user_data.organisation_name.unwrap(),
			user_id,
		)
		.await?;
		let organisation_id = organisation_id.as_bytes();

		let domain_id = service::add_domain_to_organisation(
			connection,
			user_data.org_domain_name.as_ref().unwrap(),
			organisation_id,
		)
		.await?
		.as_bytes()
		.to_vec();

		db::add_organisation_email_for_user(
			connection,
			user_id,
			user_data.org_email_local.as_ref().unwrap(),
			&domain_id,
		)
		.await?;

		welcome_email_to = Some(format!(
			"{}@{}",
			user_data.org_email_local.unwrap(),
			user_data.org_domain_name.unwrap()
		));
		if let Some((email_local, domain_id)) = user_data
			.backup_email_local
			.as_ref()
			.zip(user_data.backup_email_domain_id.as_ref())
		{
			backup_email_to = Some(format!(
				"{}@{}",
				email_local,
				db::get_personal_domain_by_id(connection, &domain_id)
					.await?
					.status(500)?
					.name
			));
			backup_phone_number_to = None;
		} else if let Some((phone_country_code, phone_number)) = user_data
			.backup_phone_country_code
			.as_ref()
			.zip(user_data.backup_phone_number.as_ref())
		{
			let country = db::get_phone_country_by_country_code(
				connection,
				&phone_country_code,
			)
			.await?
			.status(500)?;
			backup_phone_number_to =
				Some(format!("+{}{}", country.phone_code, phone_number));
			backup_email_to = None;
		} else {
			log::error!(
				"Got neither backup email, nor backup phone number while signing up user: {}",
				user_data.username
			);
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}
	} else {
		if let Some((email_local, domain_id)) = user_data
			.backup_email_local
			.as_ref()
			.zip(user_data.backup_email_domain_id.as_ref())
		{
			welcome_email_to = Some(format!(
				"{}@{}",
				email_local,
				db::get_personal_domain_by_id(connection, &domain_id)
					.await?
					.status(500)?
					.name
			));
			backup_phone_number_to = None;
		} else if let Some((phone_country_code, phone_number)) = user_data
			.backup_phone_country_code
			.as_ref()
			.zip(user_data.backup_phone_number.as_ref())
		{
			let country = db::get_phone_country_by_country_code(
				connection,
				&phone_country_code,
			)
			.await?
			.status(500)?;
			backup_phone_number_to =
				Some(format!("+{}{}", country.phone_code, phone_number));
			welcome_email_to = None;
		} else {
			log::error!(
				"Got neither backup email, nor backup phone number while signing up user: {}",
				user_data.username
			);
			return Err(Error::empty()
				.status(500)
				.body(error!(SERVER_ERROR).to_string()));
		}

		backup_email_to = None;
	}

	// add personal organisation
	let personal_organisation_name = service::get_personal_org_name(username);
	service::create_organisation(
		connection,
		&personal_organisation_name,
		user_id,
	)
	.await?;

	db::delete_user_to_be_signed_up(connection, &user_data.username).await?;

	let (jwt, login_id, refresh_token) =
		sign_in_user(connection, user_id, &config).await?;
	let response = JoinUser {
		jwt,
		login_id,
		refresh_token,
		welcome_email_to,
		backup_email_to,
		backup_phone_number_to,
	};
	Ok(response)
}
