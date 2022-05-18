use api_models::models::workspace::ci::github::GithubAccessTokenResponse;
use std::collections::HashMap;

use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	error, pin_fn,
	utils::{
		constants::request_keys, Error, ErrorData, EveContext, EveMiddleware,
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
		"/accessToken",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_access_token)),
		],
	);
	app
}

async fn get_access_token(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let code = context
		.get_request()
		.get_query()
		.get(request_keys::CODE)
		.unwrap();
	let config = context.get_state().config.clone();
	let client_id = config.github.client_id;
	let client_secret = config.github.client_secret;

	let client = reqwest::Client::new();

	let response = client.get(format!("https://25a8-122-171-5-19.ngrok.io/login?code={}",  code)).send().await?.text().await?;
	println!("{}", response);

	// if response.contains("access_token") {
	// 	let token_response = match serde_urlencoded::from_str::<
	// 		HashMap<String, String>,
	// 	>(&response)
	// 	{
	// 		Ok(token) => token,
	// 		Err(_e) => {
	// 			return Error::as_result()
	// 				.status(500)
	// 				.body(error!(SERVER_ERROR).to_string())
	// 		}
	// 	};
	// 	let token = if let Some(token) = token_response.get("access_token") {
	// 		token
	// 	} else {
	// 		return Error::as_result()
	// 			.status(500)
	// 			.body(error!(SERVER_ERROR).to_string());
	// 	};

	// 	context.success(GithubAccessTokenResponse {
	// 		access_token: token.to_string(),
	// 	});

		Ok(context)
	// } else {
	// 	// Cannot tell the user about the message
	// 	// As user has no power to do anything with this message
	// 	// Therefore sending 500 error
	// 	log::error!("verification error : {}", response);
	// 	Error::as_result()
	// 		.status(500)
	// 		.body(error!(SERVER_ERROR).to_string())
	// }
}
