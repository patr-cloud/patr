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

use crate::{
	app::{create_eve_app, App},
	db::{self, UserWebLogin},
	error,
	models::google::{GoogleAccessToken, GoogleUserInfo},
	pin_fn,
	routes,
	service,
	utils::{Error, ErrorData, EveContext, EveMiddleware},
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
		"/identify",
		[EveMiddleware::CustomFunction(pin_fn!(login_with_google))],
	);
	app.post(
		"/callback",
		[EveMiddleware::CustomFunction(pin_fn!(oauth_callback))],
	);

	app
}

async fn login_with_google(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let client_id = context.get_state().config.google.client_id.clone();
	let auth_url = context.get_state().config.google.auth_url.clone();
	let scope = context.get_state().config.google.scope.clone();
	let redirect_url = context.get_state().config.google.redirect_url.clone();

	let state = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	let state_value = context.get_state().config.google.state.clone();

	context
		.get_redis_connection()
		.set(format!("googleOAuthState:{}", state), state_value)
		.await?;

	let oauth_url =
		format!("{auth_url}?client_id={client_id}&scope={scope}&state={state}&response_type=code&redirect_uri={redirect_url}&access_type=offline");

	context.success(GoogleAuthResponse { oauth_url });
	Ok(context)
}

async fn oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GoogleAuthCallbackRequest {
		code,
		state,
		register,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let state_value = context.get_state().config.google.state.clone();

	// Check if the state is correct and request is not forged
	let redis_google_state: Option<String> = context
		.get_redis_connection()
		.get(format!("googleOAuthState:{}", state))
		.await?;

	if !redis_google_state
		.map(|value| value == state_value)
		.unwrap_or(false)
	{
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
	}

	let callback_url = context.get_state().config.google.callback_url.clone();
	let user_agent = context.get_header("User-Agent").unwrap();
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
			(
				"redirect_uri",
				context.get_state().config.google.redirect_url.clone(),
			),
			("grant_type", "authorization_code".to_string()),
			("code", code),
		])
		.header(ACCEPT, "application/json")
		.header(CONTENT_TYPE, "application/x-www-form-urlencoded")
		.header(USER_AGENT, user_agent)
		.header(CONTENT_LENGTH, "0")
		.send()
		.await?
		.error_for_status()?
		.json::<GoogleAccessToken>()
		.await?;

	log::trace!("Getting user information");
	let user_info_url = context.get_state().config.google.user_info_url.clone();
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

	match register {
		true => {
			let user_exist = db::get_user_by_email(
				context.get_database_connection(),
				&email,
			)
			.await?;
			if let Some(_user) = user_exist {
				Error::as_result()
					.status(404)
					.body(error!(EMAIL_TAKEN).to_string())?
			} else {
				let (username, _) = service::split_email_with_domain_id(
					context.get_database_connection(),
					&email,
				)
				.await?;
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
			}
		}
		false => {
			let user_exists = db::get_user_by_email(
				context.get_database_connection(),
				&email,
			)
			.await?;
			if let Some(user) = user_exists {
				let ip_address = routes::get_request_ip_address(&context);
				let user_agent =
					context.get_header("user-agent").unwrap_or_default();
				let config = context.get_state().config.clone();
				log::trace!("Get access token for user sign in");
				let (
					UserWebLogin { login_id, .. },
					access_token,
					refresh_token,
				) = service::sign_in_user(
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
					.body(error!(EMAIL_NOT_FOUND).to_string())?
			}
		}
	}
}
