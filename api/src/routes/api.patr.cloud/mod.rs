use eve_rs::{App as EveApp, Context};

use crate::{
	app::{create_eve_app, App},
	utils::{ErrorData, EveContext, EveMiddleware},
};

mod auth;
mod oauth;
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
	sub_app.use_sub_app("/oauth", oauth::create_sub_app(app));
	sub_app.use_sub_app("/user", user::create_sub_app(app));
	sub_app.use_sub_app("/workspace", workspace::create_sub_app(app));
	sub_app.use_sub_app("/webhook", webhook::create_sub_app(app));

	sub_app
}

pub fn get_request_ip_address(context: &EveContext) -> String {
	let cf_connecting_ip = context.get_header("CF-Connecting-IP");
	let x_real_ip = context.get_header("X-Real-IP");
	let x_forwarded_for =
		context.get_header("X-Forwarded-For").and_then(|value| {
			value.split(',').next().map(|ip| ip.trim().to_string())
		});
	let ip = context.get_ip().to_string();

	cf_connecting_ip
		.or(x_real_ip)
		.or(x_forwarded_for)
		.unwrap_or(ip)
}
