use crate::{
	app::{create_eve_app, App},
	db,
	models::{
		access_token_data::AccessTokenData,
		errors::{errors, messages},
	},
	pin_fn,
	utils::{constants::request_keys, get_current_time, EveContext, EveMiddleware},
};

use argon2::Config;
use express_rs::{App as EveApp, Context, Error, NextHandler};
use serde_json::{json, Value};

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(app);

	app.get(
		"/sign-in",
		&[EveMiddleware::CustomFunction(pin_fn!(sign_in))],
	);

	app
}

async fn sign_in(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: errors::WRONG_PARAMETERS,
			request_keys::MESSAGE: messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user_id = if let Some(Value::String(user_id)) = body.get(request_keys::USER_ID) {
		user_id
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: errors::WRONG_PARAMETERS,
			request_keys::MESSAGE: messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let password = if let Some(Value::String(password)) = body.get(request_keys::PASSWORD) {
		password
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: errors::WRONG_PARAMETERS,
			request_keys::MESSAGE: messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user = if let Some(user) = db::get_user(context.get_db_connection(), user_id).await? {
		user
	} else {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: errors::USER_NOT_FOUND,
			request_keys::MESSAGE: messages::USER_NOT_FOUND
		}));
		return Ok(context);
	};

	let success = argon2::verify_raw(
		password.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&user.password,
		&Config::default(),
	)?;

	if !success {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: errors::INVALID_PASSWORD,
			request_keys::MESSAGE: messages::INVALID_PASSWORD
		}));
		return Ok(context);
	}

	// generate JWT
	let iat = get_current_time();
	let exp = iat + (1000 * 3600 * 24 * 3); // 3 days

	let token_data = AccessTokenData {
		iss: String::from("https://api.bytesonus.com"),
		aud: String::from("https://*.bytesonus.com"),
		iat,
		typ: String::from("accessToken"),
		exp,
	};
	let jwt = token_data.to_string(&context.get_state().config.jwt_secret)?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt
	}));
	Ok(context)
}
