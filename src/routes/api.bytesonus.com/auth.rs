use crate::{
	app::{create_eve_app, App},
	db,
	models::{
		access_token_data::AccessTokenData,
		errors::{error_ids, error_messages},
	},
	pin_fn,
	utils::{constants::request_keys, get_current_time, validator, EveContext, EveMiddleware},
};

use argon2::Variant;
use async_std::task;
use express_rs::{App as EveApp, Context, Error, NextHandler};
use job_scheduler::Uuid;
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{json, Value};

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(app);

	app.post(
		"/sign-in",
		&[EveMiddleware::CustomFunction(pin_fn!(sign_in))],
	);
	app.post(
		"/sign-up",
		&[EveMiddleware::CustomFunction(pin_fn!(sign_up))],
	);
	app.get(
		"/access-token",
		&[EveMiddleware::CustomFunction(pin_fn!(get_access_token))],
	);
	app.get(
		"/email-available",
		&[EveMiddleware::CustomFunction(pin_fn!(is_email_available))],
	);
	app.get(
		"/username-available",
		&[EveMiddleware::CustomFunction(pin_fn!(
			is_username_available
		))],
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
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user_id = if let Some(Value::String(user_id)) = body.get(request_keys::USER_ID) {
		user_id
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let password = if let Some(Value::String(password)) = body.get(request_keys::PASSWORD) {
		password
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user = if let Some(user) =
		db::get_user_by_username_or_email(context.get_db_connection(), user_id).await?
	{
		user
	} else {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::USER_NOT_FOUND,
			request_keys::MESSAGE: error_messages::USER_NOT_FOUND
		}));
		return Ok(context);
	};

	let success = argon2::verify_raw(
		password.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&user.password,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	if !success {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::INVALID_PASSWORD,
			request_keys::MESSAGE: error_messages::INVALID_PASSWORD
		}));
		return Ok(context);
	}

	// generate JWT
	let iat = get_current_time();
	let exp = iat + (1000 * 3600 * 24 * 3); // 3 days

	let token_data = AccessTokenData::new(iat, exp);
	let jwt = token_data.to_string(context.get_state().config.jwt_secret.as_str())?;
	let refresh_token = Uuid::new_v4();

	db::add_user_login(
		context.get_db_connection(),
		refresh_token.as_bytes().to_vec(),
		iat + (1000 * 60 * 60 * 24 * 30), // 30 days
		user.id,
		iat,
		iat,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string()
	}));
	Ok(context)
}

async fn sign_up(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let username = if let Some(Value::String(username)) = body.get(request_keys::USERNAME) {
		username
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let email = if let Some(Value::String(email)) = body.get(request_keys::EMAIL) {
		email
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let password = if let Some(Value::String(password)) = body.get(request_keys::PASSWORD) {
		password
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	if !validator::is_username_valid(username) {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::INVALID_USERNAME,
			request_keys::MESSAGE: error_messages::INVALID_USERNAME
		}));
		return Ok(context);
	}

	if !validator::is_email_valid(email) {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::INVALID_EMAIL,
			request_keys::MESSAGE: error_messages::INVALID_EMAIL
		}));
		return Ok(context);
	}

	if !validator::is_password_valid(password) {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::PASSWORD_TOO_WEAK,
			request_keys::MESSAGE: error_messages::PASSWORD_TOO_WEAK
		}));
		return Ok(context);
	}

	if db::get_user_by_username(context.get_db_connection(), username)
		.await?
		.is_some()
	{
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::USERNAME_TAKEN,
			request_keys::MESSAGE: error_messages::USERNAME_TAKEN
		}));
		return Ok(context);
	}

	if db::get_user_by_email(context.get_db_connection(), email)
		.await?
		.is_some()
	{
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::EMAIL_TAKEN,
			request_keys::MESSAGE: error_messages::EMAIL_TAKEN
		}));
		return Ok(context);
	}

	let join_token = rand::thread_rng()
		.sample_iter(Alphanumeric)
		.take(40)
		.collect::<String>();
	let token_expiry = get_current_time() + (1000 * 60 * 60 * 24); // 24 hours
	let password = argon2::hash_raw(
		password.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;
	let token_hash = argon2::hash_raw(
		join_token.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;

	db::set_user_email_to_be_verified(
		context.get_db_connection(),
		email,
		username,
		&password,
		&token_hash,
		token_expiry,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));

	task::spawn(async move {
		// TODO send email with join_token as a token
		println!("Join token for user: {}", join_token);
	});

	Ok(context)
}

async fn get_access_token(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let refresh_token = if let Some(header) = context.get_header("Authorization") {
		header
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let refresh_token = if let Ok(uuid) = Uuid::parse_str(&refresh_token) {
		uuid
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let user_login = db::get_user_login(
		context.get_db_connection(),
		refresh_token.as_bytes().to_vec(),
	)
	.await?;

	if user_login.is_none() {
		context.json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::EMAIL_TOKEN_NOT_FOUND,
			request_keys::MESSAGE: error_messages::EMAIL_TOKEN_NOT_FOUND
		}));
		return Ok(context);
	}
	let user_login = user_login.unwrap();

	if user_login.token_expiry < get_current_time() {
		// Token has expired
		context.status(401).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::UNAUTHORIZED,
			request_keys::MESSAGE: error_messages::UNAUTHORIZED,
		}));
		return Ok(context);
	}

	// TODO get roles and permissions of user for rbac here
	// use that info to populate the data in the token_data

	let iat = get_current_time();
	let exp = iat + (1000 * 60 * 60 * 24 * 3); // 3 days
	let token_data = AccessTokenData::new(iat, exp);
	let refresh_token = token_data.to_string(&context.get_state().config.jwt_secret)?;

	db::set_refresh_token_expiry(
		context.get_db_connection(),
		refresh_token.as_bytes().to_vec(),
		iat,
		exp,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: refresh_token
	}));
	Ok(context)
}

async fn is_email_available(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let email = if let Some(Value::String(email)) = body.get(request_keys::EMAIL) {
		email
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	if !validator::is_email_valid(email) {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::INVALID_EMAIL,
			request_keys::MESSAGE: error_messages::INVALID_EMAIL
		}));
		return Ok(context);
	}

	let user = db::get_user_by_email(context.get_db_connection(), email).await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: user.is_none()
	}));
	Ok(context)
}

async fn is_username_available(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let body = if let Some(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	let username = if let Some(Value::String(username)) = body.get(request_keys::USERNAME) {
		username
	} else {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::WRONG_PARAMETERS,
			request_keys::MESSAGE: error_messages::WRONG_PARAMETERS
		}));
		return Ok(context);
	};

	if !validator::is_username_valid(username) {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error_ids::INVALID_USERNAME,
			request_keys::MESSAGE: error_messages::INVALID_USERNAME
		}));
		return Ok(context);
	}

	let user = db::get_user_by_username(context.get_db_connection(), username).await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: user.is_none()
	}));
	Ok(context)
}
