use crate::{
	app::{create_eve_app, App},
	db,
	models::{
		error,
		rbac::{self, permissions},
	},
	pin_fn,
	utils::{constants::request_keys, EveContext, EveMiddleware},
};

use eve_rs::{App as EveApp, Context, Error, NextHandler};
use serde_json::json;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(app);

	app.get(
		"/:organisationId/info",
		&[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_organisation_info)),
		],
	);
	app.get(
		"/:organisationId/domains",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::VIEW,
				api_macros::closure_as_pinned_box!(|mut context| {
					let org_id_string = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&org_id_string);
					if organisation_id.is_err() {
						context.status(400).json(json!({
							request_keys::SUCCESS: false,
							request_keys::ERROR: error::id::WRONG_PARAMETERS,
							request_keys::MESSAGE: error::message::WRONG_PARAMETERS
						}));
						return Ok((context, None));
					}
					let organisation_id = organisation_id.unwrap();

					let resource = db::get_resource_for_organisation(
						context.get_db_connection(),
						&organisation_id,
					)
					.await?;

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(
				get_domains_for_organisation
			)),
		],
	);
	// TODO add domains, etc

	app
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

	if !access_token_data.orgs.contains_key(&org_id_string)
		&& access_token_data.user.id != god_user_id
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

async fn get_domains_for_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let organisation_id =
		hex::decode(&context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let domains = db::get_domains_for_organisation(
		context.get_db_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|domain| {
		let id = hex::encode(domain.id);
		json!({
			request_keys::ID: id,
			request_keys::NAME: domain.name,
			request_keys::VERIFIED: domain.is_verified,
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DOMAINS: domains,
	}));
	Ok(context)
}
