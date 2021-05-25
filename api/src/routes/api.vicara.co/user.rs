use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::{json, Value};
use tokio::task;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	pin_fn,
	service,
	utils::{
		constants::request_keys,
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
	app.post(
		"/change-password",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(change_password)),
		],
	);
	app.get("/logins", []); // TODO list all logins here
	app.get("/logins/:loginId/info", []); // TODO list all information about a particular login here
	app.delete("/logins/:loginId", []); // TODO delete a particular login ID and invalidate it
	app.get(
		"/:username/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_user_info_by_username)),
		],
	);
	app
}

async fn get_user_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let user_id = context.get_token_data().unwrap().user.id.clone();
	let user =
		db::get_user_by_user_id(context.get_database_connection(), &user_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	context.json(json!({
		request_keys::SUCCESS : true,
		request_keys::USERNAME : user.username,
		request_keys::FIRST_NAME : user.first_name,
		request_keys::LAST_NAME : user.last_name,
		request_keys::BIRTHDAY : user.dob,
		request_keys::BIO : user.bio,
		request_keys::LOCATION : user.location,
		request_keys::CREATED : user.created,
	}));
	Ok(context)
}

async fn get_user_info_by_username(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let username = context.get_param(request_keys::USERNAME).unwrap().clone();

	let user_data =
		db::get_user_by_username(context.get_database_connection(), &username)
			.await?
			.status(400)
			.body(error!(PROFILE_NOT_FOUND).to_string())?;

	let mut data = serde_json::to_value(user_data)?;
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

	let first_name = body
		.get(request_keys::FIRST_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let last_name = body
		.get(request_keys::LAST_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let dob = body
		.get(request_keys::BIRTHDAY)
		.map(|value| match value {
			Value::String(value) => value
				.parse::<u64>()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()),
			Value::Number(num) => {
				if let Some(num) = num.as_u64() {
					Ok(num)
				} else if let Some(num) = num.as_i64() {
					Ok(num as u64)
				} else {
					Err(Error::empty()
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string()))
				}
			}
			_ => Err(Error::empty()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())),
		})
		.transpose()?;

	let bio = body
		.get(request_keys::BIO)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let location = body
		.get(request_keys::LOCATION)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let dob_string = dob.map(|value| value.to_string());
	let dob_str = dob_string.as_deref();

	// If no parameters to update
	first_name
		.or(last_name)
		.or(dob_str)
		.or(bio)
		.or(location)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::update_user_data(
		context.get_database_connection(),
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

	let email_address = body
		.get(request_keys::EMAIL)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::add_personal_email_to_be_verified_for_user(
		context.get_database_connection(),
		email_address,
		&user_id,
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

	let email_address = body
		.get(request_keys::EMAIL)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let otp = body
		.get(request_keys::VERIFICATION_TOKEN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::verify_personal_email_address_for_user(
		context.get_database_connection(),
		&user_id,
		email_address,
		otp,
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
		context.get_database_connection(),
		&user_id,
	)
	.await?
	.into_iter()
	.map(|org| {
		json!({
			request_keys::ID: org.id.encode_hex::<String>(),
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

async fn change_password(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let new_password = body
		.get(request_keys::NEW_PASSWORD)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let password = body
		.get(request_keys::PASSWORD)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user =
		db::get_user_by_user_id(context.get_database_connection(), &user_id)
			.await?
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

	service::change_password_for_user(
		context.get_database_connection(),
		&user_id,
		password,
		new_password,
	)
	.await?;

	if let Some((backup_email_local, backup_email_domain_id)) =
		user.backup_email_local.zip(user.backup_email_domain_id)
	{
		let config = context.get_state().config.clone();
		let email = format!(
			"{}@{}",
			backup_email_local,
			db::get_personal_domain_by_id(
				context.get_database_connection(),
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
