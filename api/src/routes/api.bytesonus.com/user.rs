use crate::{
	app::{create_eve_app, App},
	db,
	models::error,
	pin_fn,
	routes::api_bytesonus_com::middlewares::token_authenticator,
	utils::{constants::request_keys, EveContext, EveMiddleware},
};

use eve_rs::{App as EveApp, Context, Error, NextHandler};
use serde_json::{json, Value};

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(app);

	app.get(
		"/info",
		&[
			EveMiddleware::CustomFunction(token_authenticator()),
			EveMiddleware::CustomFunction(pin_fn!(get_user_info)),
		],
	);
	app.post(
		"/info",
		&[
			EveMiddleware::CustomFunction(token_authenticator()),
			EveMiddleware::CustomFunction(pin_fn!(update_user_info)),
		],
	);

	app
}

async fn get_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let mut data = serde_json::to_value(
		context.get_token_data().as_ref().unwrap().user.clone(),
	)?;
	data.as_object_mut().unwrap().remove("id");

	context.json(data);
	Ok(context)
}

async fn update_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let first_name: Option<&str> = match body.get(request_keys::FIRST_NAME) {
		Some(Value::String(first_name)) => Some(first_name),
		None => None,
		_ => {
			context.status(400).json(json!({
				request_keys::SUCCESS: false,
				request_keys::ERROR: error::id::WRONG_PARAMETERS,
				request_keys::MESSAGE: error::message::WRONG_PARAMETERS
			}));
			return Ok(context);
		}
	};

	let last_name: Option<&str> = match body.get(request_keys::LAST_NAME) {
		Some(Value::String(last_name)) => Some(last_name),
		None => None,
		_ => {
			context.status(400).json(json!({
				request_keys::SUCCESS: false,
				request_keys::ERROR: error::id::WRONG_PARAMETERS,
				request_keys::MESSAGE: error::message::WRONG_PARAMETERS
			}));
			return Ok(context);
		}
	};

	let dob: Option<&str> = match body.get(request_keys::BIRTHDAY) {
		Some(Value::String(dob)) => Some(dob),
		None => None,
		_ => {
			context.status(400).json(json!({
				request_keys::SUCCESS: false,
				request_keys::ERROR: error::id::WRONG_PARAMETERS,
				request_keys::MESSAGE: error::message::WRONG_PARAMETERS
			}));
			return Ok(context);
		}
	};

	let bio: Option<&str> = match body.get(request_keys::BIO) {
		Some(Value::String(bio)) => Some(bio),
		None => None,
		_ => {
			context.status(400).json(json!({
				request_keys::SUCCESS: false,
				request_keys::ERROR: error::id::WRONG_PARAMETERS,
				request_keys::MESSAGE: error::message::WRONG_PARAMETERS
			}));
			return Ok(context);
		}
	};

	let location: Option<&str> = match body.get(request_keys::LOCATION) {
		Some(Value::String(location)) => Some(location),
		None => None,
		_ => {
			context.status(400).json(json!({
				request_keys::SUCCESS: false,
				request_keys::ERROR: error::id::WRONG_PARAMETERS,
				request_keys::MESSAGE: error::message::WRONG_PARAMETERS
			}));
			return Ok(context);
		}
	};

	if first_name
		.or(last_name)
		.or(dob)
		.or(bio)
		.or(location)
		.is_none()
	{
		// No parameters to update
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	}

	db::update_user_data(
		context.get_db_connection(),
		first_name,
		last_name,
		dob,
		bio,
		location,
	)
	.await?;

	Ok(context)
}
