use ::redis::AsyncCommands;
use api_models::models::auth::{
	GitHubAccessTokenResponse,
	GitHubUserEmailResponse,
	GithubAuthCallbackRequest,
	GithubLoginResponse,
	LoginResponse,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use http::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::{
	app::{create_eve_app, App},
	db::{self, UserLogin},
	error,
	pin_fn,
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
		[EveMiddleware::CustomFunction(pin_fn!(identify_with_github))],
	);
	app.post(
		"/callback",
		[EveMiddleware::CustomFunction(pin_fn!(oauth_callback))],
	);
	app.post(
		"/register",
		[EveMiddleware::CustomFunction(pin_fn!(register_with_github))],
	);

	app
}

async fn identify_with_github(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let client_id = context.get_state().config.github.client_id.clone();
	let auth_url = context.get_state().config.github.auth_url.clone();

	let scope = context.get_state().config.github.scope.clone();
	let state = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(8)
		.map(char::from)
		.collect::<String>();

	// TODO: find a way to make it dynamic
	let state_value = context.get_state().config.github.state.clone();

	context
		.get_redis_connection()
		.set(format!("githubOAuthState:{}", state), state_value)
		.await?;

	let oauth_url =
		format!("{auth_url}?client_id={client_id}&scope={scope}&state={state}");
	context.success(GithubLoginResponse { oauth_url });
	Ok(context)
}

async fn oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// Register of type bool where if true then use for registering user else login
	let GithubAuthCallbackRequest { code, state, register, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let callback_url = context.get_state().config.github.callback_url.clone();

	// Check if the state is correct and not forged
	let redis_github_state: Option<String> = context
		.get_redis_connection()
		.get(format!("githubOAuthState:{}", state))
		.await?;

	let state = context.get_state().config.github.state.clone();
	if !redis_github_state
		.map(|value| value == state)
		.unwrap_or(false)
	{
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
	}

	let user_agent = context
		.get_header("User-Agent")
		.unwrap_or_else(|| "patr".to_string());

	let GitHubAccessTokenResponse { access_token, .. } =
		reqwest::Client::builder()
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
			.error_for_status()?
			.json::<GitHubAccessTokenResponse>()
			.await?;

	let user_email_api =
		context.get_state().config.github.user_email_api.clone();

	let user_emails = reqwest::Client::builder()
		.build()?
		.get(user_email_api)
		.header(AUTHORIZATION, format!("token {}", access_token))
		.header(USER_AGENT, user_agent)
		.send()
		.await?
		.error_for_status()?
		.json::<Vec<GitHubUserEmailResponse>>()
		.await?;
	
	match register {
		true => {
			// Iterate and return email that is not yet associated with patr
			let register_email = Vec::new();
			for email in user_emails.iter(){
				let user_exist = db::get_user_by_email(context.get_database_connection(), &email).await?;
				let oauth_user_exit = db::get_oauth_user_by_email(context.get_database_connection(), &email).await?;
				if user_exist.is_none() && oauth_user_exit.is_none(){
					register_email.push(email);
				}
			}
			//Send this to frontend and let user choose what email to register with.
			context.success(GithubLoginResponse { register_email });
			Ok(context)
		}
		false => {
			// Iterate and return email that is associated with patr
			let login_email = Vec::new();
			for email in user_emails.iter(){
				let user_exist = db::get_user_by_email(context.get_database_connection(), &email).await?;
				let oauth_user_exit = db::get_oauth_user_by_email(context.get_database_connection(), &email).await?;
				if user_exist.is_some() || oauth_user_exit.is_some(){
					login_email.push(email);
				}
			}
			//Send this to frontend and let user choose what email to login with. Once chosen, do normal sign-in
			context.success(GithubLoginResponse { login_email });
			Ok(context)
		}
	}
}

async fn register_with_github(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GithubAuthRegisterRequest { 
		username,
		password,
		first_name,
		last_name,
		recovery_method,
		account_type,
		coupon_code,
		is_oauth,
		emails 
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let (user_to_sign_up, verification_token) = service::create_user_join_request(
		context.get_database_connection(),
		username.to_lowercase().trim(),
		&password,
		&first_name,
		&last_name,
		&account_type,
		&recovery_method,
		coupon_code.as_deref(),
		is_oauth,
	)
	.await?;

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A new user has attempted to sign-up",
	)
	.await;

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

	// TODO: Add other emails in oauth_email table

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