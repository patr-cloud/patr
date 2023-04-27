use api_models::{
	models::prelude::*,
	prelude::*,
	utils::DecodedRequest,
	Error,
};
use axum::{
	extract::{Extension, State},
	headers::UserAgent,
	Router,
	TypedHeader,
};
use chrono::{Duration, Utc};

use crate::{
	db::UserWebLogin,
	error,
	models::UserAuthenticationData,
	prelude::*,
	routes::ClientIp,
	service::get_access_token_expiry,
};

mod docker_registry;

/// This function is used to create a router for every endpoint in this file
pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.merge(docker_registry::create_sub_app(app))
		.mount_dto(sign_in)
		.mount_dto(sign_up)
		.mount_protected_dto(
			PlainTokenAuthenticator::new().disallow_api_token(),
			app.clone(),
			sign_out,
		)
		.mount_dto(join)
		.mount_dto(get_access_token)
		.mount_dto(is_email_valid)
		.mount_dto(is_username_valid)
		.mount_dto(is_coupon_valid)
		.mount_dto(forgot_password)
		.mount_dto(reset_password)
		.mount_dto(resend_otp)
		.mount_dto(list_recovery_options)
}

/// This function will enable the user to sign in into the api
async fn sign_in(
	mut connection: Connection,
	State(config): State<Config>,
	ClientIp(client_ip): ClientIp,
	user_agent: Option<TypedHeader<UserAgent>>,
	DecodedRequest {
		body: LoginRequest { user_id, password },
		path: LoginPath,
		query: (),
	}: DecodedRequest<LoginRequest>,
) -> Result<LoginResponse, Error> {
	let user_data = db::get_user_by_username_email_or_phone_number(
		&mut connection,
		user_id.to_lowercase().trim(),
	)
	.await?;

	let success = service::validate_hash(&password, &user_data.password)?;
	if !success {
		return Err(ErrorType::InvalidPassword);
	}

	let user_agent = user_agent
		.as_ref()
		.map(|ua| ua.as_str())
		.unwrap_or_default();

	let (UserWebLogin { login_id, .. }, access_token, refresh_token) =
		service::sign_in_user(
			&mut connection,
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

/// This function is used to register the user
async fn sign_up(
	mut connection: Connection,
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
		&mut connection,
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
	service::send_user_sign_up_otp(&mut connection, &user_to_sign_up, &otp)
		.await?;

	let _ = service::get_internal_metrics(
		&mut connection,
		"A new user has attempted to sign-up",
	)
	.await;

	Ok(())
}

/// This function is used to sign-out the user
async fn sign_out(
	mut connection: Connection,
	State(mut app): State<App>,
	Extension(token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: _,
		query: _,
		body: _,
	}: DecodedRequest<LogoutRequest>,
) -> Result<(), Error> {
	let login_id = token_data.login_id();
	let user_id = token_data.user_id();

	db::get_user_web_login_for_user(&mut connection, &login_id, &user_id)
		.await?;

	db::delete_user_web_login_by_id(&mut connection, &login_id, &user_id)
		.await?;

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

/// this function is used to verify and complete the user's registration
async fn join(
	mut connection: Connection,
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
		&mut connection,
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
		&mut connection,
		"A new user has completed sign-up",
	)
	.await;

	let user = db::get_user_by_username(&mut connection, &username)
		.await?
		.ok_or_else(|| anyhow::anyhow!("user_id should be valid"))?;

	if let Some((email_local, domain_id)) =
		user.recovery_email_local.zip(user.recovery_email_domain_id)
	{
		let domain = db::get_personal_domain_by_id(&mut connection, &domain_id)
			.await?
			.ok_or_else(|| anyhow::anyhow!("domain_id should be valid"))?;

		let _ = service::include_user_to_mailchimp(
			&mut connection,
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

/// This function is used to get a new access token for a currently logged in
/// user
async fn get_access_token(
	mut connection: Connection,
	State(config): State<Config>,
	ClientIp(client_ip): ClientIp,
	user_agent: Option<TypedHeader<UserAgent>>,
	DecodedRequest {
		path: RenewAccessTokenPath,
		query,
		body: (),
	}: DecodedRequest<RenewAccessTokenRequest>,
) -> Result<RenewAccessTokenResponse, Error> {
	let refresh_token = query
		.extra_headers()
		.get(http::header::AUTHORIZATION)
		.and_then(|hv| hv.to_str().ok())
		.ok_or(|| {
			ErrorType::InternalServerError(anyhow::anyhow!(
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
		service::get_user_login_for_login_id(&mut connection, &login_id)
			.await?;

	log::trace!("Upading user login");

	let ip_info =
		service::get_ip_address_info(&client_ip, &config.ipinfo_token).await?;

	let (lat, lng) = ip_info.loc.split_once(',').status(500)?;
	let (lat, lng): (f64, f64) = (lat.parse()?, lng.parse()?);

	db::update_user_web_login_last_activity_info(
		&mut connection,
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
		return Err(Error::new(ErrorType::Unauthorized));
	}

	let access_token =
		service::generate_access_token(&mut connection, &user_login, &config)
			.await?;

	Ok(RenewAccessTokenResponse { access_token })
}

/// this function is used to check if the email address is valid or not
async fn is_email_valid(
	mut connection: Connection,
	DecodedRequest {
		body: _,
		path: _,
		query: IsEmailValidRequest { email },
	}: DecodedRequest<IsEmailValidRequest>,
) -> Result<IsEmailValidResponse, Error> {
	let email_address = email.trim().to_lowercase();

	let available =
		service::is_email_allowed(&mut connection, &email_address).await?;

	Ok(IsEmailValidResponse { available })
}

/// This function is used to check if the username id valid or not
async fn is_username_valid(
	mut connection: Connection,
	DecodedRequest {
		body: _,
		path: _,
		query: IsUsernameValidRequest { username },
	}: DecodedRequest<IsUsernameValidRequest>,
) -> Result<IsUsernameValidResponse, Error> {
	let username = username.trim().to_lowercase();

	let available =
		service::is_username_allowed(&mut connection, &username).await?;

	Ok(IsUsernameValidResponse { available })
}

/// This function is used to check if the coupon is valid or not
async fn is_coupon_valid(
	mut connection: Connection,
	DecodedRequest {
		body: _,
		path: _,
		query: IsCouponValidRequest { coupon },
	}: DecodedRequest<IsCouponValidRequest>,
) -> Result<IsCouponValidResponse, Error> {
	let valid = db::get_sign_up_coupon_by_code(&mut connection, &coupon)
		.await?
		.is_some();

	Ok(IsCouponValidResponse { valid })
}

/// This function is used to recover the user's account incase the user forgets
/// the password by sending a recovery link or otp to user's registered recovery
/// email or phone number
async fn forgot_password(
	mut connection: Connection,
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
		&mut connection,
		user_id.to_lowercase().trim(),
		&preferred_recovery_option,
	)
	.await?;

	Ok(())
}

/// This function is used to reset the password of user
async fn reset_password(
	mut connection: Connection,
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
		&mut connection,
		user_id.to_lowercase().trim(),
	)
	.await?
	.status(400)
	.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	let reset_request =
		db::get_password_reset_request_for_user(&mut connection, &user.id)
			.await?
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	// check password strength
	if !validator::is_password_valid(&password) {
		ErrorType::as_result()
			.status(200)
			.body(error!(PASSWORD_TOO_WEAK).to_string())?;
	}

	const ALLOWED_ATTEMPTS_FOR_AN_OTP: i32 = 3;
	if reset_request.attempts >= ALLOWED_ATTEMPTS_FOR_AN_OTP {
		return Err(ErrorType::empty()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string()));
	}

	let success =
		service::validate_hash(&verification_token, &reset_request.token)?;

	db::update_password_reset_request_attempt_for_user(
		&mut connection,
		&user.id,
		reset_request.attempts + 1,
	)
	.await?;

	connection.commit().await?;
	// todo: for further db conn, need to get from app state

	if !success {
		ErrorType::as_result()
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;
	}

	let is_password_same = service::validate_hash(&password, &user.password)?;

	if is_password_same {
		ErrorType::as_result()
			.status(400)
			.body(error!(PASSWORD_UNCHANGED).to_string())?;
	}

	let new_password = service::hash(password.as_bytes())?;

	db::update_user_password(&mut connection, &user.id, &new_password).await?;

	db::delete_password_reset_request_for_user(&mut connection, &user.id)
		.await?;

	service::send_user_reset_password_notification(&mut connection, user)
		.await?;

	Ok(())
}

/// This function is used to generate a new otp and send it to user
async fn resend_otp(
	mut connection: Connection,
	DecodedRequest {
		query: _,
		path: _,
		body: ResendOtpRequest { username, password },
	}: DecodedRequest<ResendOtpRequest>,
) -> Result<(), Error> {
	// update database with newly generated otp
	let (user_to_sign_up, otp) = service::resend_user_sign_up_otp(
		&mut connection,
		username.to_lowercase().trim(),
		&password,
	)
	.await?;
	// send otp
	service::send_user_sign_up_otp(&mut connection, &user_to_sign_up, &otp)
		.await?;

	Ok(())
}

/// This function is used to list the recovery options the user has given
async fn list_recovery_options(
	mut connection: Connection,
	DecodedRequest {
		query: _,
		path: _,
		body: ListRecoveryOptionsRequest { user_id },
	}: DecodedRequest<ListRecoveryOptionsRequest>,
) -> Result<(), Error> {
	let user = db::get_user_by_username_email_or_phone_number(
		&mut connection,
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
					&mut connection,
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
			&mut connection,
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
