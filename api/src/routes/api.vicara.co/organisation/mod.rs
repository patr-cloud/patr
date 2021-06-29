use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
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

mod application;
mod deployment;
mod docker_registry;
mod domain;
mod portus;
#[path = "./rbac.rs"]
mod rbac_routes;

pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.get(
		"/:organisationId/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_organisation_info)),
		],
	);
	sub_app.post(
		"/:organisationId/info",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::EDIT_INFO,
				api_macros::closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(update_organisation_info)),
		],
	);
	sub_app.use_sub_app(
		"/:organisationId/application",
		application::create_sub_app(app),
	);
	sub_app.use_sub_app(
		"/:organisationId/deployment",
		deployment::create_sub_app(app),
	);
	sub_app.use_sub_app(
		"/:organisationId/docker-registry",
		docker_registry::create_sub_app(app),
	);
	sub_app.use_sub_app("/:organisationId/domain", domain::create_sub_app(app));
	sub_app.use_sub_app("/:organisationId/portus", portus::creare_sub_app(app));
	sub_app
		.use_sub_app("/:organisationId/rbac", rbac_routes::create_sub_app(app));

	sub_app.get(
		"/is-name-available",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(is_name_available)),
		],
	);
	sub_app.post(
		"/",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(create_new_organisation)),
		],
	);
	sub_app
}

async fn get_organisation_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let org_id_string = context
		.get_param(request_keys::ORGANISATION_ID)
		.unwrap()
		.clone();
	let organisation_id = hex::decode(&org_id_string)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let access_token_data = context.get_token_data().unwrap();
	let god_user_id = rbac::GOD_USER_ID.get().unwrap().as_bytes();

	if !access_token_data.orgs.contains_key(&org_id_string) &&
		access_token_data.user.id != god_user_id
	{
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	let organisation = db::get_organisation_info(
		context.get_database_connection(),
		&organisation_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ORGANISATION_ID: org_id_string,
		request_keys::NAME: organisation.name,
		request_keys::ACTIVE: organisation.active,
		request_keys::CREATED: organisation.created,
	}));
	Ok(context)
}

async fn is_name_available(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_name = context
		.get_request()
		.get_query()
		.get(request_keys::NAME)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let allowed = service::is_organisation_name_allowed(
		context.get_database_connection(),
		&organisation_name,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: allowed
	}));
	Ok(context)
}

async fn create_new_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let organisation_name = body
		.get(request_keys::ORGANISATION_NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let user_id = context.get_token_data().unwrap().user.id.clone();

	let org_id = service::create_organisation(
		context.get_database_connection(),
		&organisation_name,
		&user_id,
	)
	.await?;
	let org_id_string = org_id.as_bytes().encode_hex::<String>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ORGANISATION_ID: org_id_string
	}));
	Ok(context)
}

async fn update_organisation_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let organisation_id =
		context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let organisation_id = hex::decode(&organisation_id).unwrap();

	let name = body
		.get(request_keys::FIRST_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?
		.map(|val| val.to_lowercase());

	if name.is_none() {
		// No parameters to update
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}
	let name = name.unwrap();

	let allowed = service::is_organisation_name_allowed(
		context.get_database_connection(),
		&name,
	)
	.await?;
	if !allowed {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_ORGANISATION_NAME).to_string())?;
	}

	db::update_organisation_name(
		context.get_database_connection(),
		&organisation_id,
		&name,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
