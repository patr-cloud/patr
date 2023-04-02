use std::net::{IpAddr, Ipv4Addr};

use api_models::{models::auth::*, utils::Uuid, ErrorType};
use axum::{
	extract::State,
	middleware,
	routing::{get, post},
	Json,
	Router,
};
use chrono::{Duration, Utc};

use crate::{
	app::App,
	db::{self, UserWebLogin},
	error,
	redis,
	routes::{self, plain_token_authenticator_without_api_token},
	service::{self, get_access_token_expiry},
	utils::{constants::request_keys, validator, Error},
};

pub fn create_sub_route(app: &App) -> Router<App> {
	let router = Router::new()
		.route("/sign-in", post(sign_in))
		.route("/sign-up", post(sign_up))
		// Middleware routes
		.merge(
			Router::new()
				.route("/sign-out", post(sign_out))
				.route_layer(middleware::from_fn_with_state(
					app.clone(),
					plain_token_authenticator_without_api_token,
				)),
		)
		.route("/join", post(join))
		.route("/access-token", get(get_access_token))
		.route("/email-valid", get(is_email_valid))
		.route("/username-valid", get(is_username_valid))
		.route("/coupon-valid", get(is_coupon_valid))
		.route("/forgot-password", post(forgot_password))
		.route("/reset-password", post(reset_password))
		.route("/resend-otp", post(resend_otp))
		.route("/list-recovery-options", post(list_recovery_options))
		.route("/", post(list_recovery_options));

	router
}

async fn sign_in(
	State(app): State<App>,
	Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, Error> {
	let LoginRequest { user_id, password } = body;
	let connection = app.database.begin().await?;
	let user_data = db::get_user_by_username_email_or_phone_number(
		context.get_database_connection(),
		user_id.to_lowercase().trim(),
	)
	.await?
	.status(200)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let success = service::validate_hash(&password, &user_data.password)?;

	if !success {
		context.error(ErrorType::InvalidPassword);
		return Ok(context);
	}

	let config = context.get_state().config.clone();
	let ip_address = routes::get_request_ip_address(&context);
	let user_agent = context.get_header("user-agent").unwrap_or_default();

	let (UserWebLogin { login_id, .. }, access_token, refresh_token) =
		service::sign_in_user(
			context.get_database_connection(),
			&user_data.id,
			&ip_address
				.parse()
				.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
			&user_agent,
			&config,
		)
		.await?;

	let login = LoginResponse {
		access_token,
		login_id,
		refresh_token,
	};

	Ok(login)
}

/// # Description
/// This function is used to register the user
/// required inputs:
/// Personal account:
/// ```
/// {
///    username:
///    password:
///    accountType: personal
///    firstName:
///    lastName:
///    recoveryEmail:
///    recoveryPhoneCountryCode:
///    recoveryPhoneNumber:
/// }
/// ```
/// Business account:
/// ```
/// {
///    username:
///    password:
///    accountType: personal
///    firstName:
///    lastName:
///    recoveryEmail:
///    recoveryPhoneCountryCode:
///    recoveryPhoneNumber:
///    businessEmailLocal:
///    domain:
///    businessName:
/// }
/// ```
///
/// In above paramters the user is only allowed to add email or phone number as
/// a back up. Both of them cannot be supplied
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn sign_up(State(app): State<App>) -> Result<EveContext, Error> {
	let CreateAccountRequest {
		username,
		password,
		first_name,
		last_name,
		recovery_method,
		account_type,
		coupon_code,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let (user_to_sign_up, otp) = service::create_user_join_request(
		context.get_database_connection(),
		username.to_lowercase().trim(),
		&password,
		&first_name,
		&last_name,
		&account_type,
		&recovery_method,
		coupon_code.as_deref(),
	)
	.await?;
	// send otp
	service::send_user_sign_up_otp(
		context.get_database_connection(),
		&user_to_sign_up,
		&otp,
	)
	.await?;

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A new user has attempted to sign-up",
	)
	.await;

	context.success(CreateAccountResponse {});
	Ok(context)
}

/// # Description
/// This function is used to sign-out the user, there will be an otp sent to
/// user's recovery email or phone number
/// required inputs:
/// example: Authorization: <insert authToken>
/// auth token in the authorization headers
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn sign_out(State(app): State<App>) -> Result<EveContext, Error> {
	let login_id = context.get_token_data().unwrap().login_id().clone();
	let user_id = context.get_token_data().unwrap().user_id().clone();

	db::get_user_web_login_for_user(
		context.get_database_connection(),
		&login_id,
		&user_id,
	)
	.await?
	.status(200)
	.body(error!(TOKEN_NOT_FOUND).to_string())?;

	db::delete_user_web_login_by_id(
		context.get_database_connection(),
		&login_id,
		&user_id,
	)
	.await?;

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	redis::revoke_login_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&login_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	context.success(LogoutResponse {});
	Ok(context)
}

/// # Description
/// this function is used to verify the user's registration and register the
/// user
/// required inputs:
/// ```
/// {
///    verificationToken:
///    username:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn join(State(app): State<App>) -> Result<EveContext, Error> {
	let CompleteSignUpRequest {
		username,
		verification_token,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	let ip_address = routes::get_request_ip_address(&context);
	let user_agent = context.get_header("user-agent").unwrap_or_default();

	let join_user = service::join_user(
		context.get_database_connection(),
		&verification_token,
		username.to_lowercase().trim(),
		&ip_address
			.parse()
			.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
		&user_agent,
		&config,
	)
	.await?;

	service::send_sign_up_complete_notification(
		join_user.welcome_email_to,
		join_user.recovery_email_to,
		join_user.recovery_phone_number_to,
		&username,
	)
	.await?;

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A new user has completed sign-up",
	)
	.await;

	let user =
		db::get_user_by_username(context.get_database_connection(), &username)
			.await?
			.status(500)?;

	if let Some((email_local, domain_id)) =
		user.recovery_email_local.zip(user.recovery_email_domain_id)
	{
		let domain = db::get_personal_domain_by_id(
			context.get_database_connection(),
			&domain_id,
		)
		.await?
		.status(500)?;

		let _ = service::include_user_to_mailchimp(
			context.get_database_connection(),
			&format!("{}@{}", email_local, domain.name),
			&user.first_name,
			&user.last_name,
			&config,
		)
		.await;
	}

	context.success(CompleteSignUpResponse {
		access_token: join_user.jwt,
		login_id: join_user.login_id,
		refresh_token: join_user.refresh_token,
	});
	Ok(context)
}

/// # Description
/// This function is used to get a new access token for a currently logged in
/// user
/// required inputs:
/// refresh token in authorization header
/// example: Authorization: <insert refreshToken>
/// ```
/// {
///    loginId: email or username
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
///    accessToken:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_access_token(State(app): State<App>) -> Result<EveContext, Error> {
	let refresh_token = context
		.get_header("Authorization")
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let login_id = context
		.get_request()
		.get_query()
		.get(request_keys::LOGIN_ID)
		.and_then(|value| Uuid::parse_str(value).ok())
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let ip_address = routes::get_request_ip_address(&context);
	let user_agent = context.get_header("user-agent").unwrap_or_default();

	let config = context.get_state().config.clone();
	let user_login = service::get_user_login_for_login_id(
		context.get_database_connection(),
		&login_id,
	)
	.await?;

	log::trace!("Upading user login");
	let ip_addr = &ip_address
		.parse()
		.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));

	let ip_info =
		service::get_ip_address_info(ip_addr, &config.ipinfo_token).await?;

	let (lat, lng) = ip_info.loc.split_once(',').status(500)?;
	let (lat, lng): (f64, f64) = (lat.parse()?, lng.parse()?);

	db::update_user_web_login_last_activity_info(
		context.get_database_connection(),
		&login_id,
		&Utc::now(),
		ip_addr,
		lat,
		lng,
		&ip_info.country,
		&ip_info.region,
		&ip_info.city,
		&ip_info.timezone,
		&user_agent,
	)
	.await?;

	let success =
		service::validate_hash(&refresh_token, &user_login.refresh_token)?;

	if !success {
		Error::as_result()
			.status(200)
			.body(error!(UNAUTHORIZED).to_string())?;
	}

	let access_token = service::generate_access_token(
		context.get_database_connection(),
		&user_login,
		&config,
	)
	.await?;

	context.success(RenewAccessTokenResponse { access_token });
	Ok(context)
}

/// # Description
/// this function is used to check if the email address is valid or not
/// required inputs:
/// ```
/// {
///    email:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true false
///    available: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn is_email_valid(State(app): State<App>) -> Result<EveContext, Error> {
	let IsEmailValidRequest { email } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let email_address = email.trim().to_lowercase();

	let available = service::is_email_allowed(
		context.get_database_connection(),
		&email_address,
	)
	.await?;

	context.success(IsEmailValidResponse { available });
	Ok(context)
}

/// # Description
/// This function is used to check if the username id valid or not
/// required inputs:
/// ```
/// {
///    username:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
///    available: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn is_username_valid(
	State(app): State<App>,
) -> Result<EveContext, Error> {
	let IsUsernameValidRequest { username } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let username = username.trim().to_lowercase();

	let available = service::is_username_allowed(
		context.get_database_connection(),
		&username,
	)
	.await?;

	context.success(IsUsernameValidResponse { available });
	Ok(context)
}

/// # Description
/// This function is used to check if the coupon is valid or not
/// required inputs:
/// ```
/// {
///    username:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
///    available: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn is_coupon_valid(State(app): State<App>) -> Result<EveContext, Error> {
	let IsCouponValidRequest { coupon } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let valid = db::get_sign_up_coupon_by_code(
		context.get_database_connection(),
		&coupon,
	)
	.await?
	.is_some();

	context.success(IsCouponValidResponse { valid });
	Ok(context)
}

/// # Description
/// This function is used to recover the user's account incase the user forgets
/// the password by sending a recovery link or otp to user's registered recovery
/// email or phone number
/// required inputs:
/// ```
/// {
///    userId:
///    preferredRecoveryOption: RecoveryPhoneNumber or RecoveryEmail
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn forgot_password(State(app): State<App>) -> Result<EveContext, Error> {
	let ForgotPasswordRequest {
		user_id,
		preferred_recovery_option,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// service function should take care of otp generation and delivering the
	// otp to the preferred recovery option
	service::forgot_password(
		context.get_database_connection(),
		user_id.to_lowercase().trim(),
		&preferred_recovery_option,
	)
	.await?;

	context.success(ForgotPasswordResponse {});
	Ok(context)
}

/// # Description
/// This function is used to reset the password of user
/// required inputs:
/// ```
/// {
///    password:
///    verificationToken:
///    userId:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn reset_password(State(app): State<App>) -> Result<EveContext, Error> {
	let ResetPasswordRequest {
		user_id,
		password,
		verification_token,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user = db::get_user_by_username_email_or_phone_number(
		context.get_database_connection(),
		user_id.to_lowercase().trim(),
	)
	.await?
	.status(400)
	.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	let reset_request = db::get_password_reset_request_for_user(
		context.get_database_connection(),
		&user.id,
	)
	.await?
	.status(400)
	.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	// check password strength
	if !validator::is_password_valid(&password) {
		Error::as_result()
			.status(200)
			.body(error!(PASSWORD_TOO_WEAK).to_string())?;
	}

	const ALLOWED_ATTEMPTS_FOR_AN_OTP: i32 = 3;
	if reset_request.attempts >= ALLOWED_ATTEMPTS_FOR_AN_OTP {
		return Err(Error::empty()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string()));
	}

	let success =
		service::validate_hash(&verification_token, &reset_request.token)?;

	db::update_password_reset_request_attempt_for_user(
		context.get_database_connection(),
		&user.id,
		reset_request.attempts + 1,
	)
	.await?;
	context.commit_database_transaction().await?;

	if !success {
		Error::as_result()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;
	}

	let is_password_same = service::validate_hash(&password, &user.password)?;

	if is_password_same {
		Error::as_result()
			.status(400)
			.body(error!(PASSWORD_UNCHANGED).to_string())?;
	}

	let new_password = service::hash(password.as_bytes())?;

	db::update_user_password(
		context.get_database_connection(),
		&user.id,
		&new_password,
	)
	.await?;

	db::delete_password_reset_request_for_user(
		context.get_database_connection(),
		&user.id,
	)
	.await?;

	service::send_user_reset_password_notification(
		context.get_database_connection(),
		user,
	)
	.await?;

	context.success(ResetPasswordResponse {});
	Ok(context)
}

/// # Description
/// This function is used to generate a new otp and send it to user
/// required inputs:
/// ```
/// {
///    username:
///    password:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    otp:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn resend_otp(State(app): State<App>) -> Result<EveContext, Error> {
	let ResendOtpRequest { username, password } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// update database with newly generated otp
	let (user_to_sign_up, otp) = service::resend_user_sign_up_otp(
		context.get_database_connection(),
		username.to_lowercase().trim(),
		&password,
	)
	.await?;
	// send otp
	service::send_user_sign_up_otp(
		context.get_database_connection(),
		&user_to_sign_up,
		&otp,
	)
	.await?;

	context.success(ResendOtpResponse {});
	Ok(context)
}

/// # Description
/// This function is used to list the recovery options the user has given
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// {
///    userId: username or email
/// }
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// {
///    recoveryPhoneNumber:
///    recoveryEmail:
///    success: true or false
/// }
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_recovery_options(
	State(app): State<App>,
) -> Result<EveContext, Error> {
	let ListRecoveryOptionsRequest { user_id } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user = db::get_user_by_username_email_or_phone_number(
		context.get_database_connection(),
		user_id.to_lowercase().trim(),
	)
	.await?
	.status(404)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let recovery_email =
		if let (Some(recovery_email_local), Some(recovery_email_domain_id)) =
			(user.recovery_email_local, user.recovery_email_domain_id)
		{
			Some(format!(
				"{}@{}",
				service::mask_email_local(&recovery_email_local),
				db::get_personal_domain_by_id(
					context.get_database_connection(),
					&recovery_email_domain_id
				)
				.await?
				.status(500)?
				.name
			))
		} else {
			None
		};

	let recovery_phone_number = if let Some(recovery_phone_number) =
		user.recovery_phone_number
	{
		let phone_number = service::mask_phone_number(&recovery_phone_number);
		let country_code = db::get_phone_country_by_country_code(
			context.get_database_connection(),
			&user.recovery_phone_country_code.unwrap(),
		)
		.await?
		.status(500)
		.body(error!(INVALID_PHONE_NUMBER).to_string())?;

		Some(format!("+{}{}", country_code.phone_code, phone_number))
	} else {
		None
	};

	context.success(ListRecoveryOptionsResponse {
		recovery_email,
		recovery_phone_number,
	});
	Ok(context)
}
