pub use api_macros::ErrorResponse;
use eve_rs::{App as EveApp, Context};

use crate::{
	app::{create_eve_app, App},
	models::error::{id as ErrId, message as ErrMsg},
	utils::{errors::AsErrorResponse, ErrorData, EveContext, EveMiddleware},
};

mod auth;
mod user;
mod webhook;
mod workspace;

#[derive(ErrorResponse)]
pub enum APIError {
	#[error(
		status = 404,
		id = ErrId::WRONG_PARAMETERS,
		message = ErrMsg::WRONG_PARAMETERS,
	)]
	WrongParameters,

	#[error(
		status = 200,
		id = ErrId::USER_NOT_FOUND,
		message = ErrMsg::USER_NOT_FOUND
	)]
	UserNotFound,

	#[error(
		status = 200,
		id = ErrId::TOKEN_NOT_FOUND,
		message = ErrMsg::TOKEN_NOT_FOUND
	)]
	TokenNotFound,

	#[error(
		status = 200,
		id = ErrId::UNAUTHORIZED,
		message = ErrMsg::UNAUTHORIZED
	)]
	Unauthorized,

	#[error(
		status = 400,
		id = ErrId::EMAIL_TOKEN_NOT_FOUND,
		message = ErrMsg::EMAIL_TOKEN_NOT_FOUND
	)]
	EmailTokenNotFound,

	#[error(
		status = 404,
		id = ErrId::USER_NOT_FOUND,
		message = ErrMsg::USER_NOT_FOUND
	)]
	UserNotFound404,

	#[error(
		status = 500,
		id = ErrId::INVALID_PHONE_NUMBER,
		message = ErrMsg::INVALID_PHONE_NUMBER
	)]
	InvalidPhoneNumber,
}

#[derive(ErrorResponse)]
pub enum CodedError {
	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::INVALID_CLIENT_ID,
	)]
	InvalidClientId,

	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::INVALID_OFFLINE_TOKEN,
	)]
	InvalidOfflineToken,

	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::OFFLINE_TOKEN_NOT_FOUND,
	)]
	OfflineTokenNotFound,

	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::SERVICE_NOT_FOUND,
	)]
	ServiceNotFound,

	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::INVALID_SERVICE,
	)]
	InvalidService,

	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::AUTHORIZATION_PARSE_ERROR,
	)]
	AuthorizationParseError,

	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::AUTHORIZATION_NOT_FOUND,
	)]
	AuthorizationNotFound,

	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::USERNAME_NOT_FOUND,
	)]
	UsernameNotFound,

	#[error(
		status = 400,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::PASSWORD_NOT_FOUND,
	)]
	PasswordNotFound,

	#[error(
		status = 401,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::USER_NOT_FOUND,
	)]
	UserNotFound,

	#[error(
		status = 401,
		code = ErrId::UNAUTHORIZED,
		message = ErrMsg::INVALID_PASSWORD,
	)]
	InvalidPassword,
}

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
