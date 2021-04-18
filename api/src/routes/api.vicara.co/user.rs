use eve_rs::{App as EveApp, Context, NextHandler};
use serde_json::{json, Value};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{db_mapping::UserEmailAddress, ExposedUserData},
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		get_current_time,
		validator,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	app.get(
		"/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_user_info)),
		],
	);
	app.post(
		"/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(update_user_info)),
		],
	);
	app.get(
		"/:username/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_user_info_by_username)),
		],
	);
	app.post(
		"/add-email-address",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(add_email_address)),
		],
	);
	app.post(
		"/verify-email-address",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(verify_email_address)),
		],
	);
	app.get(
		"/organisations",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_organisations_for_user)),
		],
	);

	app
}

async fn get_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let mut data = serde_json::to_value(
		context.get_token_data().as_ref().unwrap().user.clone(),
	)?;
	let object = data.as_object_mut().unwrap();
	object.remove(request_keys::ID);
	object.insert(request_keys::SUCCESS.to_string(), true.into());

	context.json(data);
	Ok(context)
}

async fn get_user_info_by_username(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let username = context.get_param(request_keys::USERNAME).unwrap().clone();

	let user_data =
		db::get_user_by_username(context.get_mysql_connection(), &username)
			.await?;

	if user_data.is_none() {
		context.status(400).json(error!(PROFILE_NOT_FOUND));
		return Ok(context);
	}
	let user_data = user_data.unwrap();
	let data = ExposedUserData {
		id: vec![],
		username: user_data.username,
		first_name: user_data.first_name,
		last_name: user_data.last_name,
		created: user_data.created,
	};

	let mut data = serde_json::to_value(data)?;
	let object = data.as_object_mut().unwrap();
	object.remove(request_keys::ID);
	object.insert(request_keys::SUCCESS.to_string(), true.into());

	context.json(json!(data));
	Ok(context)
}

async fn update_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let first_name: Option<&str> = match body.get(request_keys::FIRST_NAME) {
		Some(Value::String(first_name)) => Some(first_name),
		None => None,
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	};

	let last_name: Option<&str> = match body.get(request_keys::LAST_NAME) {
		Some(Value::String(last_name)) => Some(last_name),
		None => None,
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	};

	let dob: Option<&str> = match body.get(request_keys::BIRTHDAY) {
		Some(Value::String(dob)) => Some(dob),
		None => None,
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	};

	let bio: Option<&str> = match body.get(request_keys::BIO) {
		Some(Value::String(bio)) => Some(bio),
		None => None,
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	};

	let location: Option<&str> = match body.get(request_keys::LOCATION) {
		Some(Value::String(location)) => Some(location),
		None => None,
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
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
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	}

	db::update_user_data(
		context.get_mysql_connection(),
		first_name,
		last_name,
		dob,
		bio,
		location,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn add_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let email_address =
		if let Some(Value::String(email)) = body.get(request_keys::EMAIL) {
			email
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	if !validator::is_email_valid(email_address) {
		context.json(error!(INVALID_EMAIL));
		return Ok(context);
	}

	if db::get_user_by_email(context.get_mysql_connection(), &email_address)
		.await?
		.is_some()
	{
		context.json(error!(EMAIL_TAKEN));
		return Ok(context);
	}

	let otp = service::generate_new_otp();
	let otp = format!("{}-{}", &otp[..3], &otp[3..]);

	let token_expiry = get_current_time() + (1000 * 60 * 60 * 2); // 2 hours
	let verification_token = argon2::hash_raw(
		otp.as_bytes(),
		context.get_state().config.password_pepper.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;
	let user_id = context.get_token_data().unwrap().user.id.clone();

	db::add_personal_email_to_be_verified_for_user(
		context.get_mysql_connection(),
		&email_address,
		&user_id,
		&verification_token,
		token_expiry,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn verify_email_address(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let email =
		if let Some(Value::String(email)) = body.get(request_keys::EMAIL) {
			email
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let otp = if let Some(Value::String(token)) =
		body.get(request_keys::VERIFICATION_TOKEN)
	{
		token
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};
	let user_id = context.get_token_data().unwrap().user.id.clone();

	let email_verification_data =
		db::get_personal_email_to_be_verified_for_user(
			context.get_mysql_connection(),
			&user_id,
			&email,
		)
		.await?;

	if email_verification_data.is_none() {
		context.status(400).json(error!(EMAIL_TOKEN_NOT_FOUND));
		return Ok(context);
	}
	let email_verification_data = email_verification_data.unwrap();

	let success = argon2::verify_raw(
		otp.as_bytes(),
		context.get_state().config.password_pepper.as_bytes(),
		&email_verification_data.verification_token_hash,
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;
	if !success {
		context.json(error!(EMAIL_TOKEN_NOT_FOUND));
		return Ok(context);
	}

	if email_verification_data.verification_token_expiry < get_current_time() {
		context.json(error!(EMAIL_TOKEN_EXPIRED));
		return Ok(context);
	}

	let email_address =
		UserEmailAddress::Personal(email_verification_data.email_address);

	db::add_email_for_user(
		context.get_mysql_connection(),
		&user_id,
		email_address,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn get_organisations_for_user(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();
	let organisations = db::get_all_organisations_for_user(
		context.get_mysql_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|org| {
		json!({
			request_keys::ID: hex::encode(org.id),
			request_keys::NAME: org.name,
			request_keys::ACTIVE: org.active,
			request_keys::CREATED: org.created
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ORGANISATIONS: organisations
	}));
	Ok(context)
}
