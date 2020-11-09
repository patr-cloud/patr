use crate::{
	app::{create_eve_app, App},
	db,
	models::{error, rbac},
	pin_fn,
	utils::{constants::request_keys, EveContext, EveMiddleware},
};
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use serde_json::json;

mod domain;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app.clone());

	sub_app.get(
		"/:organisationId/info",
		&[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_organisation_info)),
		],
	);

	sub_app.use_sub_app("/domain", domain::create_sub_app(app));

	sub_app
}

async fn get_organisation_info(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let org_id_string = context
		.get_param(request_keys::ORGANISATION_ID)
		.unwrap()
		.clone();
	let organisation_id = hex::decode(&org_id_string);
	if organisation_id.is_err() {
		context.status(400).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::WRONG_PARAMETERS,
			request_keys::MESSAGE: error::message::WRONG_PARAMETERS
		}));
		return Ok(context);
	}
	let organisation_id = organisation_id.unwrap();
	let access_token_data = context.get_token_data().unwrap();
	let god_user_id = rbac::GOD_USER_ID.get().unwrap().as_bytes();

	if !access_token_data.orgs.contains_key(&org_id_string) &&
		access_token_data.user.id != god_user_id
	{
		context.status(404).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::RESOURCE_DOES_NOT_EXIST,
			request_keys::MESSAGE: error::message::RESOURCE_DOES_NOT_EXIST,
		}));
		return Ok(context);
	}

	let organisation = db::get_organisation_info(
		context.get_db_connection(),
		&organisation_id,
	)
	.await?;

	if let Some(organisation) = organisation {
		context.json(json!({
			request_keys::SUCCESS: true,
			request_keys::ORGANISATION_ID: organisation_id,
			request_keys::NAME: organisation.name,
			request_keys::ACTIVE: organisation.active,
			request_keys::CREATED: organisation.created,
		}));
	} else {
		context.status(500).json(json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::SERVER_ERROR,
			request_keys::MESSAGE: error::message::SERVER_ERROR,
		}));
	}

	Ok(context)
}
