use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;
use tokio::task;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	pin_fn,
	service,
	utils::{
		constants::{request_keys, ResourceOwnerType},
		mailer,
		Error,
		ErrorData,
		EveContext,
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

	let success = service::validate_hash(&password, &user_data.password)?;

	if !success {
		context.json(error!(INVALID_PASSWORD));
		return Ok(context);
	}

	let config = context.get_state().config.clone();
	let (jwt, refresh_token) = service::sign_in_user(
		context.get_mysql_connection(),
		&user_data.id,
		&config,
	)
	.await?;

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
		.map(|a| a.parse::<ResourceOwnerType>().ok())
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

	let backup_email = body
		.get(request_keys::BACKUP_EMAIL)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let backup_phone_country_code = body
		.get(request_keys::BACKUP_PHONE_COUNTRY_CODE)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let backup_phone_number = body
		.get(request_keys::BACKUP_PHONE_NUMBER)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let org_email_local = body
		.get(request_keys::ORGANISATION_EMAIL_LOCAL)
		.map(|param| {
			param
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let org_domain_name = body
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

	let otp = service::create_user_join_request(
		context.get_mysql_connection(),
		username,
		account_type,
		password,
		(first_name, last_name),
		backup_email,
		backup_phone_country_code,
		backup_phone_number,
		org_email_local,
		org_domain_name,
		organisation_name,
	)
	.await?;

	if let Some(email) = backup_email {
		let config = context.get_state().config.clone();
		let email = email.to_string();

		task::spawn_blocking(|| {
			mailer::send_email_verification_mail(config, email, otp);
		});
	}
	if let Some((_country_code, _phone_number)) =
		backup_phone_country_code.zip(backup_phone_number)
	{
		// TODO implement this
		panic!("Sending OTPs through phone numbers aren't handled yet");
	}

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn join(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let otp = body
		.get(request_keys::VERIFICATION_TOKEN)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let username = body
		.get(request_keys::USERNAME)
		.map(|param| param.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	let result = service::join_user(
		context.get_mysql_connection(),
		&config,
		otp,
		username,
	)
	.await?;
	let (
		jwt,
		refresh_token,
		welcome_email_to,
		backup_email_to,
		backup_phone_number_to,
	) = result;

	if let Some(welcome_email_to) = welcome_email_to {
		task::spawn_blocking(|| {
			mailer::send_sign_up_completed_mail(config, welcome_email_to);
		});
	}

	if let Some(backup_email) = backup_email_to {
		let config = context.get_state().config.clone();
		task::spawn_blocking(|| {
			mailer::send_backup_registration_mail(config, backup_email);
		});
	}

	if let Some(_phone_number) = backup_phone_number_to {
		// TODO implement this
		panic!("Sending OTPs through phone numbers aren't handled yet");
	}

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ACCESS_TOKEN: jwt,
		request_keys::REFRESH_TOKEN: refresh_token.to_simple().to_string().to_lowercase()
	}));
	Ok(context)
}

async fn get_access_token(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let refresh_token = context
		.get_header("Authorization")
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let login_id = hex::decode(
		context
			.get_param(request_keys::LOGIN_ID)
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?,
	)
	.status(400)
	.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();
	let user_login = service::get_user_login_for_login_id(
		context.get_mysql_connection(),
		&login_id,
	)
	.await?;
	let success =
		service::validate_hash(&refresh_token, &user_login.refresh_token)?;

	if !success {
		Error::as_result()
			.status(200)
			.body(error!(UNAUTHORIZED).to_string())?;
	}

	let access_token = service::generate_access_token(
		context.get_mysql_connection(),
		&config,
		&user_login,
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

	let email_address = query
		.get(request_keys::EMAIL)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

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

	let username = query
		.get(request_keys::USERNAME)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let allowed =
		service::is_username_allowed(context.get_mysql_connection(), username)
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

	let user_id = body
		.get(request_keys::USER_ID)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();
	let (otp, backup_email) =
		service::forgot_password(context.get_mysql_connection(), user_id)
			.await?;

	task::spawn_blocking(|| {
		mailer::send_password_reset_requested_mail(config, backup_email, otp);
	});

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

async fn reset_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let new_password = body
		.get(request_keys::PASSWORD)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let token = body
		.get(request_keys::VERIFICATION_TOKEN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let username = body
		.get(request_keys::USERNAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user =
		db::get_user_by_username(context.get_mysql_connection(), username)
			.await?
			.status(400)
			.body(error!(EMAIL_TOKEN_NOT_FOUND).to_string())?;

	let config = context.get_state().config.clone();

	service::reset_password(
		context.get_mysql_connection(),
		new_password,
		token,
		&user.id,
	)
	.await?;

	if let Some((backup_email_local, backup_email_domain_id)) =
		user.backup_email_local.zip(user.backup_email_domain_id)
	{
		let email = format!(
			"{}@{}",
			backup_email_local,
			db::get_personal_domain_by_id(
				context.get_mysql_connection(),
				&backup_email_domain_id
			)
			.await?
			.status(500)?
			.name
		);
		task::spawn_blocking(|| {
			mailer::send_password_changed_notification_mail(config, email);
		});
	}

	if let Some((_phone_country_code, _phone_number)) =
		user.backup_phone_country_code.zip(user.backup_phone_number)
	{
		// TODO implement this
		panic!("Sending OTPs through phone numbers aren't handled yet");
	}

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
