use std::net::{IpAddr, Ipv4Addr};

use ::redis::AsyncCommands;
use api_models::{
	models::auth::{
		CreateAccountResponse,
		GithubAuthCallbackRequest,
		GithubIdentifyResponse,
		LoginResponse,
		RecoveryMethod,
		SignUpAccountType,
	},
	utils::Personal,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use url::Url;

use crate::{
	app::{create_eve_app, App},
	db::{self, UserWebLogin},
	error,
	models::github::{GitHubAccessToken, GitHubUserEmail, GitHubUserInfo},
	pin_fn,
	routes,
	service,
	utils::{
		constants::github_oauth,
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
			authorize_with_github
		))],
	);
	app.post(
		"/callback",
		[EveMiddleware::CustomFunction(pin_fn!(oauth_callback))],
	);

	app
}

async fn authorize_with_github(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let client_id = context.get_state().config.github.client_id.clone();
	let auth_url = github_oauth::AUTH_URL.to_owned();
	let scope = github_oauth::SCOPE.to_owned();

	let state = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	context
		.get_redis_connection()
		.set("githubOAuthState", state.clone())
		.await?;

	let oauth_url = Url::parse(&format!(
		"{auth_url}?client_id={client_id}&scope={scope}&state={state}"
	))?;
	context.success(GithubIdentifyResponse {
		oauth_url: oauth_url.to_string(),
	});
	Ok(context)
}

async fn oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GithubAuthCallbackRequest {
		code,
		state,
		username,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let callback_url = github_oauth::CALLBACK_URL.to_owned();

	// Check if the state is correct and not forged
	let redis_github_state: Option<String> = context
		.get_redis_connection()
		.get("githubOAuthState")
		.await?;

	if !redis_github_state
		.map(|value| value == state)
		.unwrap_or(false)
	{
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
	}

	log::trace!("Getting access token");
	let GitHubAccessToken { access_token } = reqwest::Client::builder()
		.build()?
		.post(callback_url)
		.query(&[
			(
				"client_id",
				context.get_state().config.github.client_id.clone(),
			),
			(
				"client_secret",
				context.get_state().config.github.client_secret.clone(),
			),
			("code", code),
		])
		.header(ACCEPT, "application/json")
		.send()
		.await?
		.json::<GitHubAccessToken>()
		.await?;

	log::trace!("Getting user's primary email");
	let user_email_api = github_oauth::USER_EMAIL_API.to_owned();
	let user_email = reqwest::Client::builder()
		.build()?
		.get(user_email_api)
		.header(AUTHORIZATION, format!("token {}", access_token))
		.header(USER_AGENT, "patr".to_string())
		.send()
		.await?
		.error_for_status()?
		.json::<Vec<GitHubUserEmail>>()
		.await?;

	let primary_email = user_email.into_iter().find(|email| email.primary);
	let email = if let Some(email) = primary_email {
		email.email
	} else {
		Error::as_result()
			.status(404)
			.body(error!(EMAIL_NOT_FOUND).to_string())?
	};

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
		log::trace!("Getting user information");
		let user_info_api = github_oauth::USER_INFO_API.to_owned();
		let user_info = reqwest::Client::builder()
			.build()?
			.get(user_info_api.clone())
			.header(AUTHORIZATION, format!("token {}", access_token))
			.header(USER_AGENT, "patr".to_string())
			.send()
			.await?
			.error_for_status()?
			.json::<GitHubUserInfo>()
			.await?;

		let mut name = user_info.name.split(' ');
		// TODO Better error message if first name not found
		let first_name = name
			.next()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
		let last_name = name.next().unwrap_or_default();
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
