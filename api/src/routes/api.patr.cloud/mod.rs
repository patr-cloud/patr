use api_models::models::GetVersionResponse;
use eve_rs::{App as EveApp, NextHandler};

use crate::{
	app::{create_eve_app, App},
	pin_fn,
	utils::{constants, Error, ErrorData, EveContext, EveMiddleware},
};

mod auth;
mod user;
mod webhook;
mod workspace;

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions. This file
/// contains major enpoints of the API, and all other endpoints will come under
/// this
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
	let mut sub_app = create_eve_app(app);

	sub_app.use_sub_app("/auth", auth::create_sub_app(app));
	sub_app.use_sub_app("/user", user::create_sub_app(app));
	sub_app.use_sub_app("/workspace", workspace::create_sub_app(app));
	sub_app.use_sub_app("/webhook", webhook::create_sub_app(app));
	sub_app.get(
		"/version",
		[EveMiddleware::CustomFunction(pin_fn!(get_version_number))],
	);

	sub_app
}

async fn get_version_number(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	context.success(GetVersionResponse {
		version: constants::DATABASE_VERSION.to_string(),
	});
	Ok(context)
}
