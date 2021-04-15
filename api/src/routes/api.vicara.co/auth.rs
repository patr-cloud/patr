use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::{json, Value};
use tokio::task;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	pin_fn,
	service,
	utils::{
		constants::{request_keys, AccountType},
		mailer,
		AsErrorData,
		ErrorData,
		EveContext,
		EveError as Error,
		EveMiddleware,
	},
};

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(&app);

	app.post(
		"/sign-in",
		[EveMiddleware::CustomFunction(pin_fn!(sign_in))],
	);
	app.post(
		"/sign-up",
		[EveMiddleware::CustomFunction(pin_fn!(sign_up))],
	);
	app.post("/join", [EveMiddleware::CustomFunction(pin_fn!(join))]);
	app.get(
		"/access-token",
		[EveMiddleware::CustomFunction(pin_fn!(get_access_token))],
	);
	app.get(
		"/email-valid",
		[EveMiddleware::CustomFunction(pin_fn!(is_email_valid))],
	);
	app.get(
		"/username-valid",
		[EveMiddleware::CustomFunction(pin_fn!(is_username_valid))],
	);
	app.post(
		"/forgot-password",
		[EveMiddleware::CustomFunction(pin_fn!(forgot_password))],
	);
	app.post(
		"/reset-password",
		[EveMiddleware::CustomFunction(pin_fn!(reset_password))],
	);

	app
}

async fn sign_in(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = body
		.get(request_keys::USER_ID)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let password = body
		.get(request_keys::PASSWORD)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_data = db::get_user_by_username_or_email(
		context.get_mysql_connection(),
		&user_id,
	)
	.await?
	.status(200)
	.body(error!(USER_NOT_FOUND).to_string())?;

	let success = service::validate_hash(
		password.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&user_data.password,
	)?;

	if !success {
		context.json(error!(INVALID_PASSWORD));
		return Ok(context);
	}

	let app_config = context.get_state().config.clone();
	let (jwt, refresh_token) = service::sign_in(
		context.get_mysql_connection(),
		user_data,
		&app_config,
	)
	.await
	.commit_transaction(false)?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string().to_lowercase()
	}));
	Ok(context)
}

async fn sign_up(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let username = body
		.get(request_keys::USERNAME)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let email = body
		.get(request_keys::EMAIL)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let password = body
		.get(request_keys::PASSWORD)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let account_type = body
		.get(request_keys::ACCOUNT_TYPE)
		.map(|param| param.as_str())
		.flatten()
		.map(|a| a.parse::<AccountType>().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let first_name = body
		.get(request_keys::FIRST_NAME)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let last_name = body
		.get(request_keys::LAST_NAME)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let domain_name = body
		.get(request_keys::DOMAIN)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let organisation_name = body
		.get(request_keys::ORGANISATION_NAME)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let backup_email = body
		.get(request_keys::BACKUP_EMAIL)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let config = context.get_state().config.clone();

	let otp = service::create_user_join_request(
		context.get_mysql_connection(),
		&config,
		username,
		email,
		password,
		account_type,
		(first_name, last_name),
		(domain_name, organisation_name, backup_email),
	)
	.await?;
	let email = email.to_string();

	task::spawn_blocking(|| {
		mailer::send_email_verification_mail(config, email, otp);
	});

	context.json(json!({
		request_keys::SUCCESS: true,
	}));
	Ok(context)
}

async fn join(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let otp = if let Some(Value::String(token)) =
		body.get(request_keys::VERIFICATION_TOKEN)
	{
		token
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let username = if let Some(Value::String(username)) =
		body.get(request_keys::USERNAME)
	{
		username
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let config = context.get_state().config.clone();

	let status =
		service::join(context.get_mysql_connection(), config, otp, username)
			.await;
	if let Err(err) = status {
		context.json(json!(err));
		return Ok(context);
	}
	let (jwt, refresh_token, welcome_email_to, backup_email_notification_to) =
		status.unwrap();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string().to_lowercase()
	}));

	let config = context.get_state().config.clone();
	task::spawn_blocking(|| {
		mailer::send_sign_up_completed_mail(config, welcome_email_to);
	});

	if let Some(backup_email) = backup_email_notification_to {
		let config = context.get_state().config.clone();
		task::spawn_blocking(|| {
			mailer::send_backup_registration_mail(config, backup_email);
		});
	}

	Ok(context)
}

async fn get_access_token(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let refresh_token =
		if let Some(header) = context.get_header("Authorization") {
			header
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let config = context.get_state().config.clone();
	let access_token = service::get_access_token_data(
		context.get_mysql_connection(),
		config,
		&refresh_token,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: access_token
	}));
	Ok(context)
}

async fn is_email_valid(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query().clone();

	let email_address = if let Some(email) = query.get(request_keys::EMAIL) {
		email
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let allowed = service::is_email_allowed(
		context.get_mysql_connection(),
		email_address,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: allowed
	}));
	Ok(context)
}

async fn is_username_valid(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let query = context.get_request().get_query().clone();

	let username = if let Some(username) = query.get(request_keys::USERNAME) {
		username
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let allowed = service::is_username_allowed(
		context.get_mysql_connection(),
		username,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: allowed
	}));
	Ok(context)
}

async fn forgot_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id =
		if let Some(Value::String(user_id)) = body.get(request_keys::USER_ID) {
			user_id
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let config = context.get_state().config.clone();
	let status = service::forgot_password(
		context.get_mysql_connection(),
		config,
		user_id,
	)
	.await;

	if let Err(err) = status {
		context.json(err);
		return Ok(context);
	}
	let otp = status.unwrap();

	context.json(json!({
		request_keys::SUCCESS: true,
		"otp" : otp
	}));
	Ok(context)
}

async fn reset_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let new_password = if let Some(Value::String(password)) =
		body.get(request_keys::PASSWORD)
	{
		password
	} else {
		log::debug!("password");
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};
	let token = if let Some(Value::String(token)) =
		body.get(request_keys::VERIFICATION_TOKEN)
	{
		token
	} else {
		log::debug!("token");
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};
	let user_id =
		if let Some(Value::String(user_id)) = body.get(request_keys::USER_ID) {
			user_id
		} else {
			log::debug!("id");
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};
	let user_id = if let Ok(user_id) = hex::decode(user_id) {
		user_id
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let config = context.get_state().config.clone();
	let pool = context.get_state().mysql.clone();

	let status = service::reset_password(
		context.get_mysql_connection(),
		&config,
		new_password,
		token,
		&user_id,
	)
	.await;
	if let Err(err) = status {
		context.json(json!(err));
		return Ok(context);
	}

	task::spawn(async move {
		let mut connection = pool
			.begin()
			.await
			.expect("unable to begin transaction from connection");
		let user = db::get_user_by_user_id(&mut connection, &user_id)
			.await
			.expect("unable to get user data")
			.expect("user data for that user_id was None");

		task::spawn_blocking(|| {
			mailer::send_password_changed_notification_mail(
				config,
				user.backup_email,
			);
		});
	});

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
