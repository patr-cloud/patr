use std::net::{IpAddr, Ipv4Addr};

use ::redis::AsyncCommands;
use api_models::{
	models::auth::{
		CreateAccountResponse,
		GoogleAuthCallbackRequest,
		GoogleAuthResponse,
		LoginResponse,
		RecoveryMethod,
		SignUpAccountType,
	},
	utils::Personal,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::header::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, USER_AGENT};
use url::Url;

use crate::{
	app::{create_eve_app, App},
	db::{self, UserWebLogin},
	error,
	models::google::{GoogleAccessToken, GoogleUserInfo},
	pin_fn,
	routes,
	service,
	utils::{
		constants::google_oauth,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

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
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	app.post(
		"/authorize",
		[EveMiddleware::CustomFunction(pin_fn!(
			authorize_with_google
		))],
	);
	app.post(
		"/callback",
		[EveMiddleware::CustomFunction(pin_fn!(oauth_callback))],
	);

	app
}

async fn authorize_with_google(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let client_id = context.get_state().config.google.client_id.clone();
	let auth_url = google_oauth::AUTH_URL.to_owned();
	let scope = google_oauth::SCOPE.to_owned();
	let redirect_url = google_oauth::REDIRECT_URL.to_owned();

	let state = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	context
		.get_redis_connection()
		.set("googleOAuthState", state.clone())
		.await?;

	let oauth_url = Url::parse(
		&format!("{auth_url}?client_id={client_id}&scope={scope}&state={state}&response_type=code&redirect_uri={redirect_url}&access_type=offline")
	)?;

	context.success(GoogleAuthResponse {
		oauth_url: oauth_url.to_string(),
	});
	Ok(context)
}

async fn oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GoogleAuthCallbackRequest {
		code,
		state,
		username,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// Check if the state is correct and request is not forged
	let redis_google_state: Option<String> = context
		.get_redis_connection()
		.get("googleOAuthState")
		.await?;

	if !redis_google_state
		.map(|value| value == state)
		.unwrap_or(false)
	{
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
	}

	if let Some(username) = username {
		let username_exist = db::get_user_by_username(
			context.get_database_connection(),
			&username,
		)
		.await?;
		if let Some(_username_exists) = username_exist {
			Error::as_result()
				.status(404)
				.body(error!(USERNAME_TAKEN).to_string())?
		}
		let GoogleUserInfo { name, email } =
			get_user_info(&context, code).await?;
		let mut user_name = name.split(" ");
		// TODO Better error message if first name not found
		let first_name = user_name
			.next()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
		let last_name = user_name.next().unwrap_or_default();
		let password = "".to_string();
		let account_type = SignUpAccountType::Personal {
			account_type: Personal,
		};
		let recovery_method = RecoveryMethod::Email {
			recovery_email: email,
		};
		log::trace!("Creating join request for user");
		let (user_to_sign_up, otp) = service::create_user_join_request(
			context.get_database_connection(),
			username.to_lowercase().trim(),
			&password,
			first_name,
			last_name,
			&account_type,
			&recovery_method,
			None,
			true,
		)
		.await?;

		log::trace!("Sending otp to user's primary mail");
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
		log::trace!("Registration success");
		context.success(CreateAccountResponse {});
		Ok(context)
	} else {
		let GoogleUserInfo { email, .. } =
			get_user_info(&context, code).await?;
		let user_exist =
			db::get_user_by_email(context.get_database_connection(), &email)
				.await?;
		if let Some(user) = user_exist {
			let ip_address = routes::get_request_ip_address(&context);
			let user_agent =
				context.get_header("user-agent").unwrap_or_default();
			let config = context.get_state().config.clone();
			log::trace!("Get access token for user sign in");
			let (UserWebLogin { login_id, .. }, access_token, refresh_token) =
				service::sign_in_user(
					context.get_database_connection(),
					&user.id,
					&ip_address
						.parse()
						.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
					&user_agent,
					&config,
				)
				.await?;
			log::trace!("Login success");
			context.success(LoginResponse {
				login_id,
				access_token,
				refresh_token,
			});

			Ok(context)
		} else {
			Error::as_result()
				.status(404)
				.body(error!(INVALID_USERNAME).to_string())?
		}
	}
}

async fn get_user_info(
	context: &EveContext,
	code: String,
) -> Result<GoogleUserInfo, Error> {
	let callback_url = google_oauth::CALLBACK_URL.to_owned();
	log::trace!("Getting access token");
	let access_token = reqwest::Client::builder()
		.build()?
		.post(callback_url)
		.query(&[
			(
				"client_id",
				context.get_state().config.google.client_id.clone(),
			),
			(
				"client_secret",
				context.get_state().config.google.client_secret.clone(),
			),
			("redirect_uri", google_oauth::REDIRECT_URL.to_owned()),
			("grant_type", "authorization_code".to_string()),
			("code", code),
		])
		.header(ACCEPT, "application/json")
		.header(CONTENT_TYPE, "application/x-www-form-urlencoded")
		.header(USER_AGENT, "patr".to_string())
		.header(CONTENT_LENGTH, "0")
		.send()
		.await?
		.error_for_status()?
		.json::<GoogleAccessToken>()
		.await?;
	log::trace!("Getting user information");
	let user_info_url = google_oauth::USER_INFO_URL.to_owned();
	let GoogleUserInfo { name, email } = reqwest::Client::builder()
		.build()?
		.get(user_info_url)
		.header(
			"Authorization",
			format!("Bearer {}", access_token.access_token),
		)
		.header(ACCEPT, "application/json")
		.send()
		.await?
		.error_for_status()?
		.json::<GoogleUserInfo>()
		.await?;
	Ok(GoogleUserInfo { name, email })
}
