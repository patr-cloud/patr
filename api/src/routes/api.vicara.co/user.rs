use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	pin_fn,
	service,
	utils::{
		constants::request_keys,
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

	let dob: Option<&str> = body
		.get(request_keys::BIRTHDAY)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let bio: Option<&str> = body
		.get(request_keys::BIO)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let location: Option<&str> = body
		.get(request_keys::LOCATION)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	// If no parameters to update
	first_name
		.or(last_name)
		.or(dob)
		.or(bio)
		.or(location)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

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

	let email_address = body
		.get(request_keys::EMAIL)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let user_id = context.get_token_data().unwrap().user.id.clone();

	service::add_personal_email_to_be_verified_for_user(
		context.get_mysql_connection(),
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
		context.get_mysql_connection(),
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
		context.get_mysql_connection(),
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
