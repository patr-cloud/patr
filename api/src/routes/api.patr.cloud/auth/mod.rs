use api_models::{
	models::auth::{
		CompleteSignUpRequest,
		CompleteSignUpResponse,
		CreateAccountRequest,
		ForgotPasswordRequest,
		IsCouponValidRequest,
		IsCouponValidResponse,
		IsEmailValidRequest,
		IsEmailValidResponse,
		IsUsernameValidRequest,
		IsUsernameValidResponse,
		ListRecoveryOptionsRequest,
		ListRecoveryOptionsResponse,
		LoginRequest,
		LoginResponse,
		LogoutRequest,
		RenewAccessTokenRequest,
		RenewAccessTokenResponse,
		ResendOtpRequest,
		ResetPasswordRequest,
	},
	prelude::*,
	utils::DecodedRequest,
	Error,
};
use axum::{
	extract::{Extension, State},
	headers::UserAgent,
	http::HeaderMap,
	middleware,
	Router,
	TypedHeader,
};
use chrono::{Duration, Utc};

use crate::{
	app::{App, Config, DbTx},
	db::{self, UserWebLogin},
	error,
	models::UserAuthenticationData,
	redis,
	routes::{
		self,
		api_patr_cloud::middlewares::plain_token_authenticator,
		ClientIp,
	},
	service::{self, get_access_token_expiry},
	utils::{constants::request_keys, validator},
};

mod docker_registry;

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
/// api including the database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(app: App) -> Router<App> {
	let unauthorized_routes = Router::new()
		.mount_dto(sign_in)
		.mount_dto(sign_up)
		.mount_dto(join)
		.mount_dto(get_access_token)
		.mount_dto(is_email_valid)
		.mount_dto(is_username_valid)
		.mount_dto(is_coupon_valid)
		.mount_dto(forgot_password)
		.mount_dto(reset_password)
		.mount_dto(resend_otp)
		.mount_dto(list_recovery_options);

	let authorized_routes = Router::new().mount_dto(sign_out).layer(
		middleware::from_fn_with_state(app.clone(), plain_token_authenticator),
	);

	authorized_routes.merge(unauthorized_routes)
}

/// # Description
/// This function will enable the user to sign in into the api
/// required inputs:
/// ```
/// {
///    userId: username or email address
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
///    success: true or false
///    accessToken:
///    RefreshToken:
///    LoginId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn sign_in(
	mut db_tx: DbTx,
	State(config): State<Config>,
	ClientIp(client_ip): ClientIp,
	user_agent: Option<TypedHeader<UserAgent>>,
	DecodedRequest {
		body: LoginRequest { user_id, password },
		path: _,
		query: _,
	}: DecodedRequest<LoginRequest>,
) -> Result<LoginResponse, Error> {
	let user_data = db::get_user_by_username_email_or_phone_number(
		&mut db_tx,
		user_id.to_lowercase().trim(),
	)
	.await?
	.ok_or_else(|| Error::NotFound)?;

	let success = service::validate_hash(&password, &user_data.password)?;
	if !success {
		return Err(Error::InvalidPassword);
	}

	let user_agent = user_agent
		.as_ref()
		.map(|ua| ua.as_str())
		.unwrap_or_default();

	let (UserWebLogin { login_id, .. }, access_token, refresh_token) =
		service::sign_in_user(
			&mut db_tx,
			&user_data.id,
			&client_ip,
			&user_agent,
			&config,
		)
		.await?;

	Ok(LoginResponse {
		access_token,
		login_id,
		refresh_token,
	})
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
async fn sign_up(
	mut db_tx: DbTx,
	DecodedRequest {
		path: _,
		query: _,
		body:
			CreateAccountRequest {
				username,
				password,
				first_name,
				last_name,
				recovery_method,
				account_type,
				coupon_code,
			},
	}: DecodedRequest<CreateAccountRequest>,
) -> Result<(), Error> {
	let (user_to_sign_up, otp) = service::create_user_join_request(
		&mut db_tx,
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
	service::send_user_sign_up_otp(&mut db_tx, &user_to_sign_up, &otp).await?;

	let _ = service::get_internal_metrics(
		&mut db_tx,
		"A new user has attempted to sign-up",
	)
	.await;

	Ok(())
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
async fn sign_out(
	mut db_tx: DbTx,
	State(mut app): State<App>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: _,
		query: _,
		body: (),
	}: DecodedRequest<LogoutRequest>,
) -> Result<(), Error> {
	let login_id = token_data.login_id();
	let user_id = token_data.user_id();

	db::get_user_web_login_for_user(&mut db_tx, &login_id, &user_id)
		.await?
		.ok_or_else(|| Error::NotFound)?;

	db::delete_user_web_login_by_id(&mut db_tx, &login_id, &user_id).await?;

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	redis::revoke_login_tokens_created_before_timestamp(
		&mut app.redis,
		&login_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await
	.map_err(|err| anyhow::anyhow!(err))?;

	Ok(())
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
async fn join(
	mut db_tx: DbTx,
	State(config): State<Config>,
	ClientIp(client_ip): ClientIp,
	user_agent: Option<TypedHeader<UserAgent>>,
	DecodedRequest {
		path: _,
		query: _,
		body: CompleteSignUpRequest {
			username,
			verification_token,
		},
	}: DecodedRequest<CompleteSignUpRequest>,
) -> Result<CompleteSignUpResponse, Error> {
	let user_agent = user_agent
		.as_ref()
		.map(|ua| ua.as_str())
		.unwrap_or_default();

	let join_user = service::join_user(
		&mut db_tx,
		&verification_token,
		username.to_lowercase().trim(),
		&client_ip,
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
		&mut db_tx,
		"A new user has completed sign-up",
	)
	.await;

	let user = db::get_user_by_username(&mut db_tx, &username)
		.await?
		.ok_or_else(|| anyhow::anyhow!("user_id should be valid"))?;

	if let Some((email_local, domain_id)) =
		user.recovery_email_local.zip(user.recovery_email_domain_id)
	{
		let domain = db::get_personal_domain_by_id(&mut db_tx, &domain_id)
			.await?
			.ok_or_else(|| anyhow::anyhow!("domain_id should be valid"))?;

		let _ = service::include_user_to_mailchimp(
			&mut db_tx,
			&format!("{}@{}", email_local, domain.name),
			&user.first_name,
			&user.last_name,
			&config,
		)
		.await;
	}

	Ok(CompleteSignUpResponse {
		access_token: join_user.jwt,
		login_id: join_user.login_id,
		refresh_token: join_user.refresh_token,
	})
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
async fn get_access_token(
	mut db_tx: DbTx,
	State(config): State<Config>,
	ClientIp(client_ip): ClientIp,
	user_agent: Option<TypedHeader<UserAgent>>,
	DecodedRequest {
		path: _,
		query,
		body: _,
	}: DecodedRequest<RenewAccessTokenRequest>,
) -> Result<RenewAccessTokenResponse, Error> {
	let refresh_token = query
		.extra_headers()
		.get(http::header::AUTHORIZATION)
		.and_then(|hv| hv.to_str().ok())
		.ok_or(|| {
			Error::InternalServerError(anyhow::anyhow!(
				"Auth header should be present"
			))
		})?;

	let RenewAccessTokenRequest {
		refresh_token: _,
		login_id,
	} = query;

	let user_agent = user_agent
		.as_ref()
		.map(|ua| ua.as_str())
		.unwrap_or_default();

	let user_login =
		service::get_user_login_for_login_id(&mut db_tx, &login_id).await?;

	log::trace!("Upading user login");

	let ip_info =
		service::get_ip_address_info(&client_ip, &config.ipinfo_token).await?;

	let (lat, lng) = ip_info.loc.split_once(',').status(500)?;
	let (lat, lng): (f64, f64) = (lat.parse()?, lng.parse()?);

	db::update_user_web_login_last_activity_info(
		&mut db_tx,
		&login_id,
		&Utc::now(),
		&client_ip,
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

	let access_token =
		service::generate_access_token(&mut db_tx, &user_login, &config)
			.await?;

	Ok(RenewAccessTokenResponse { access_token })
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
async fn is_email_valid(
	mut db_tx: DbTx,
	DecodedRequest {
		body: _,
		path: _,
		query: IsEmailValidRequest { email },
	}: DecodedRequest<IsEmailValidRequest>,
) -> Result<IsEmailValidResponse, Error> {
	let email_address = email.trim().to_lowercase();

	let available =
		service::is_email_allowed(&mut db_tx, &email_address).await?;

	Ok(IsEmailValidResponse { available })
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
	mut db_tx: DbTx,
	DecodedRequest {
		body: _,
		path: _,
		query: IsUsernameValidRequest { username },
	}: DecodedRequest<IsUsernameValidRequest>,
) -> Result<IsUsernameValidResponse, Error> {
	let username = username.trim().to_lowercase();

	let available = service::is_username_allowed(&mut db_tx, &username).await?;

	Ok(IsUsernameValidResponse { available })
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
async fn is_coupon_valid(
	mut db_tx: DbTx,
	DecodedRequest {
		body: _,
		path: _,
		query: IsCouponValidRequest { coupon },
	}: DecodedRequest<IsCouponValidRequest>,
) -> Result<IsCouponValidResponse, Error> {
	let valid = db::get_sign_up_coupon_by_code(&mut db_tx, &coupon)
		.await?
		.is_some();

	Ok(IsCouponValidResponse { valid })
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
async fn forgot_password(
	mut db_tx: DbTx,
	DecodedRequest {
		query: _,
		path: _,
		body:
			ForgotPasswordRequest {
				user_id,
				preferred_recovery_option,
			},
	}: DecodedRequest<ForgotPasswordRequest>,
) -> Result<(), Error> {
	// service function should take care of otp generation and delivering the
	// otp to the preferred recovery option
	service::forgot_password(
		&mut db_tx,
		user_id.to_lowercase().trim(),
		&preferred_recovery_option,
	)
	.await?;

	Ok(())
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
async fn reset_password(
	mut db_tx: DbTx,
	DecodedRequest {
		query: _,
		path: _,
		body:
			ResetPasswordRequest {
				user_id,
				verification_token,
				password,
			},
	}: DecodedRequest<ResetPasswordRequest>,
) -> Result<(), Error> {
	let user = db::get_user_by_username_email_or_phone_number(
		&mut db_tx,
		user_id.to_lowercase().trim(),
	)
	.await?
	.status(400)
	.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	let reset_request =
		db::get_password_reset_request_for_user(&mut db_tx, &user.id)
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
		&mut db_tx,
		&user.id,
		reset_request.attempts + 1,
	)
	.await?;

	db_tx.commit().await?;
	// todo: for further db conn, need to get from app state

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

	db::update_user_password(&mut db_tx, &user.id, &new_password).await?;

	db::delete_password_reset_request_for_user(&mut db_tx, &user.id).await?;

	service::send_user_reset_password_notification(&mut db_tx, user).await?;

	Ok(())
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
async fn resend_otp(
	mut db_tx: DbTx,
	DecodedRequest {
		query: _,
		path: _,
		body: ResendOtpRequest { username, password },
	}: DecodedRequest<ResendOtpRequest>,
) -> Result<(), Error> {
	// update database with newly generated otp
	let (user_to_sign_up, otp) = service::resend_user_sign_up_otp(
		&mut db_tx,
		username.to_lowercase().trim(),
		&password,
	)
	.await?;
	// send otp
	service::send_user_sign_up_otp(&mut db_tx, &user_to_sign_up, &otp).await?;

	Ok(())
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
	mut db_tx: DbTx,
	DecodedRequest {
		query: _,
		path: _,
		body: ListRecoveryOptionsRequest { user_id },
	}: DecodedRequest<ListRecoveryOptionsRequest>,
) -> Result<(), Error> {
	let user = db::get_user_by_username_email_or_phone_number(
		&mut db_tx,
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
					&mut db_tx,
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
			&mut db_tx,
			&user.recovery_phone_country_code.unwrap(),
		)
		.await?
		.status(500)
		.body(error!(INVALID_PHONE_NUMBER).to_string())?;

		Some(format!("+{}{}", country_code.phone_code, phone_number))
	} else {
		None
	};

	Ok(ListRecoveryOptionsResponse {
		recovery_email,
		recovery_phone_number,
	})
}
