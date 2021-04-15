use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::{json, Value};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{
		constants::request_keys,
		get_current_time,
		validator,
		ErrorData,
		EveContext,
		EveError as Error,
		EveMiddleware,
	},
};

mod application;
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
					let organisation_id = hex::decode(&org_id_string);
					if organisation_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let organisation_id = organisation_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&organisation_id,
					)
					.await
					.status(500)
					.body(error!(SERVER_ERROR).to_string())?;

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
	sub_app.use_sub_app("/:organisationId/portus", portus::creare_sub_app(app));
	sub_app.use_sub_app("/:organisationId/domain", domain::create_sub_app(app));
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
	let organisation_id = hex::decode(&org_id_string);
	if organisation_id.is_err() {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	}
	let organisation_id = organisation_id.unwrap();
	let access_token_data = context.get_token_data().unwrap();
	let god_user_id = rbac::GOD_USER_ID.get().unwrap().as_bytes();

	if !access_token_data.orgs.contains_key(&org_id_string) &&
		access_token_data.user.id != god_user_id
	{
		context.status(404).json(error!(RESOURCE_DOES_NOT_EXIST));
		return Ok(context);
	}

	let organisation = db::get_organisation_info(
		context.get_mysql_connection(),
		&organisation_id,
	)
	.await?;

	if let Some(organisation) = organisation {
		context.json(json!({
			request_keys::SUCCESS: true,
			request_keys::ORGANISATION_ID: org_id_string,
			request_keys::NAME: organisation.name,
			request_keys::ACTIVE: organisation.active,
			request_keys::CREATED: organisation.created,
		}));
	} else {
		context.status(500).json(error!(SERVER_ERROR));
	}

	Ok(context)
}

async fn is_name_available(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let organisation_name =
		if let Some(Value::String(name)) = body.get(request_keys::NAME) {
			name
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	if !validator::is_organisation_name_valid(&organisation_name) {
		context.status(400).json(error!(INVALID_ORGANISATION_NAME));
		return Ok(context);
	}

	let organisation = db::get_organisation_by_name(
		context.get_mysql_connection(),
		&organisation_name,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::AVAILABLE: organisation.is_none()
	}));
	Ok(context)
}

async fn create_new_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let body = context.get_body_object().clone();

	let domain_name =
		if let Some(Value::String(domain)) = body.get(request_keys::DOMAIN) {
			domain
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let organisation_name = if let Some(Value::String(organisation_name)) =
		body.get(request_keys::ORGANISATION_NAME)
	{
		organisation_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	if !validator::is_organisation_name_valid(&organisation_name) {
		context.status(400).json(error!(INVALID_ORGANISATION_NAME));
		return Ok(context);
	}

	let organisation = db::get_organisation_by_name(
		context.get_mysql_connection(),
		&organisation_name,
	)
	.await?;

	if organisation.is_some() {
		context.status(400).json(error!(RESOURCE_EXISTS));
		return Ok(context);
	}

	let organisation_id =
		db::generate_new_resource_id(context.get_mysql_connection())
			.await
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	let organisation_id = organisation_id.as_bytes();
	let org_id_string = hex::encode(organisation_id);
	let user_id = context.get_token_data().unwrap().user.id.clone();

	db::create_orphaned_resource(
		context.get_mysql_connection(),
		organisation_id,
		&format!("Organiation: {}", organisation_name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::ORGANISATION)
			.unwrap(),
	)
	.await?;
	db::create_organisation(
		context.get_mysql_connection(),
		organisation_id,
		&organisation_name,
		&user_id,
		get_current_time(),
	)
	.await?;
	db::set_resource_owner_id(
		context.get_mysql_connection(),
		organisation_id,
		organisation_id,
	)
	.await?;

	let domain_id =
		db::generate_new_resource_id(context.get_mysql_connection())
			.await
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;
	let domain_id = domain_id.as_bytes().to_vec();

	db::create_resource(
		context.get_mysql_connection(),
		&domain_id,
		&format!("Domain: {}", domain_name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DOMAIN)
			.unwrap(),
		organisation_id,
	)
	.await?;
	db::add_domain_to_organisation(
		context.get_mysql_connection(),
		&domain_id,
		&domain_name,
	)
	.await?;

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

	let name: Option<&str> = match body.get(request_keys::FIRST_NAME) {
		Some(Value::String(first_name)) => Some(first_name),
		None => None,
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	};

	if name.is_none() {
		// No parameters to update
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	}
	let name = name.unwrap();

	if !validator::is_organisation_name_valid(&name) {
		context.status(400).json(error!(INVALID_ORGANISATION_NAME));
		return Ok(context);
	}

	db::update_organisation_name(
		context.get_mysql_connection(),
		&organisation_id,
		name,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
