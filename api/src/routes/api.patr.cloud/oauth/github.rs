use ::redis::AsyncCommands;
use api_models::models::auth::{
	GitHubAccessTokenResponse,
	GitHubUserInfoResponse,
	GithubAuthCallbackRequest,
	GithubAuthResponse,
	LoginResponse,
};
use eve_rs::{App as EveApp, AsError, NextHandler};
use http::header::ACCEPT;
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
		"/login",
		[EveMiddleware::CustomFunction(pin_fn!(login_with_github))],
	);
	app.post(
		"/callback",
		[EveMiddleware::CustomFunction(pin_fn!(oauth_callback))],
	);

	app
}

async fn login_with_github(
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

	let value = "random-unique-alpha";

	context
		.get_redis_connection()
		.set(format!("githubOAuthState:{}", state), value)
		.await?;

	let oauth_url =
		format!("{auth_url}?client_id={client_id}&scope={scope}&state={state}");
	context.success(GithubAuthResponse { oauth_url });
	Ok(context)
}

async fn oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GithubAuthCallbackRequest { code, state, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let callback_url = context.get_state().config.github.callback_url.clone();

	// Check if the state is correct
	let redis_github_state: Option<String> = context
		.get_redis_connection()
		.get(format!("githubOAuthState:{}", state))
		.await?;

	if !redis_github_state
		.map(|value| value == "random-unique-alpha")
		.unwrap_or(false)
	{
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
	}

	let GitHubAccessTokenResponse {
		access_token,
		scope: _scope,
		token_type: _token_type,
	} = reqwest::Client::builder()
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

	// Make a call to github to get the user details
	let user_info_url = context.get_state().config.github.user_info_url.clone();
	let GitHubUserInfoResponse { username, email } = reqwest::Client::builder()
		.build()?
		.get(user_info_url)
		.header("Authorization", format!("token {}", access_token))
		.header(ACCEPT, "application/json")
		.send()
		.await?
		.error_for_status()?
		.json::<GitHubUserInfoResponse>()
		.await?;

	let user_exists =
		db::get_user_by_email(context.get_database_connection(), &email)
			.await?;

	if let Some(user) = user_exists {
		db::update_user_oauth_info(
			context.get_database_connection(),
			&access_token,
			&user.id,
			&username,
			true,
		)
		.await?;

		let config = context.get_state().config.clone();
		let (UserLogin { login_id, .. }, access_token, refresh_token) =
			service::sign_in_user(
				context.get_database_connection(),
				&user.id,
				&config,
			)
			.await?;

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
