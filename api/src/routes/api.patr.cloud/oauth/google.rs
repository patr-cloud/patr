use ::redis::AsyncCommands;
use api_models::models::auth::{
	GoogleAccessTokenResponse,
	GoogleAuthCallbackRequest,
	GoogleAuthResponse,
	GoogleUserInfoResponse,
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

	let value = "random-unique-alpha";

	context
		.get_redis_connection()
		.set(format!("googleOAuthState:{}", state), value)
		.await?;

	let oauth_url =
		format!("{auth_url}?client_id={client_id}&scope={scope}&state={state}&response_type=code&redirect_url={redirect_url}");

	context.success(GoogleAuthResponse { oauth_url });
	Ok(context)
}

async fn oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GoogleAuthCallbackRequest { code, state, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let callback_url =
		context.get_state().config.google.oauth_callback_url.clone();

	// Check if the state is correct
	let redis_google_state: Option<String> = context
		.get_redis_connection()
		.get(format!("googleOAuthState:{}", state))
		.await?;

	if !redis_google_state
		.map(|value| value == "random-unique-alpha")
		.unwrap_or(false)
	{
		Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?
	}

	let GoogleAccessTokenResponse {
		access_token,
		scope: _scope,
		token_type: _token_type,
	} = reqwest::Client::builder()
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
			("code", code),
		])
		.header(ACCEPT, "application/json")
		.send()
		.await?
		.error_for_status()?
		.json::<GoogleAccessTokenResponse>()
		.await?;

	// Make a call to Google to get the user details
	// TODO change this in accordance with google's new api
	// let user_info_url =
	// context.get_state().config.google.user_info_url.clone();
	// let GoogleUserInfoResponse { username, email } =
	// reqwest::Client::builder() 	.build()?
	// 	.get(user_info_url)
	// 	.header("Authorization", format!("token {}", access_token))
	// 	.header(ACCEPT, "application/json")
	// 	.send()
	// 	.await?
	// 	.error_for_status()?
	// 	.json::<GoogleUserInfoResponse>()
	// 	.await?;

	// let user_exists =
	// 	db::get_user_by_email(context.get_database_connection(), &email)
	// 		.await?;

	// if let Some(user) = user_exists {
	// 	db::update_user_oauth_info(
	// 		context.get_database_connection(),
	// 		&access_token,
	// 		&user.id,
	// 		&username,
	// 		true,
	// 	)
	// 	.await?;

	// 	let config = context.get_state().config.clone();
	// 	let (UserLogin { login_id, .. }, access_token, refresh_token) =
	// 		service::sign_in_user(
	// 			context.get_database_connection(),
	// 			&user.id,
	// 			&config,
	// 		)
	// 		.await?;

	// 	context.success(LoginResponse {
	// 		login_id,
	// 		access_token,
	// 		refresh_token,
	// 	});

	// 	Ok(context)
	// } else {
	// 	Error::as_result()
	// 		.status(404)
	// 		.body(error!(EMAIL_NOT_FOUND).to_string())?
	// }
	Ok(context)
}
