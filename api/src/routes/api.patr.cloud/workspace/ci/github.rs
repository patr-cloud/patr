use api_models::models::workspace::ci::github::{
	GithubAuthCallbackResponse,
	GithubAuthResponse,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use reqwest::header::COOKIE;

use crate::{
	app::{create_eve_app, App},
	error,
	pin_fn,
	utils::{
		constants::request_keys,
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
///   api including the
/// database connections.
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

	app.get(
		"/oauth-callback",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(github_oauth_callback)),
		],
	);

	app.get(
		"/auth",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(connect_to_github)),
		],
	);
	app
}

async fn connect_to_github(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let client = reqwest::Client::builder()
		.redirect(reqwest::redirect::Policy::none())
		.build()?;

	let response = client
		.get(format!("{}/login", config.drone.url))
		.send()
		.await?;
	let oauth_url = response
		.headers()
		.get("location")
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let oauth_url = oauth_url.to_str()?;

	context.success(GithubAuthResponse {
		oauth_url: oauth_url.to_string(),
	});
	Ok(context)
}

async fn github_oauth_callback(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let config = context.get_state().config.clone();

	let code = context
		.get_request()
		.get_query()
		.get(request_keys::CODE)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let state = context
		.get_request()
		.get_query()
		.get(request_keys::STATE)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let client = reqwest::Client::new();
	let response = client
		.get(format!(
			"{}/login?code={}&state={}",
			config.drone.url, code, state
		))
		.header(COOKIE, format!("_oauth_state_={}", state))
		.send()
		.await?;
	if response.status() != 200 {
		return Error::as_result()
			.status(500)
			.body(error!(SERVER_ERROR).to_string());
	}

	context.success(GithubAuthCallbackResponse {});

	Ok(context)
}
