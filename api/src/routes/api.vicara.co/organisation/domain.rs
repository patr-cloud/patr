use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
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
	let mut app = create_eve_app(&app);

	// Get all domains
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(
				get_domains_for_organisation
			)),
		],
	);

	// Add a domain
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::ADD,
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
			EveMiddleware::CustomFunction(pin_fn!(add_domain_to_organisation)),
		],
	);

	// Verify a domain
	app.post(
		"/:domainId/verify",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::VERIFY,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::DOMAIN_ID).unwrap();
					let domain_id = hex::decode(&domain_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&domain_id,
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
			EveMiddleware::CustomFunction(pin_fn!(
				verify_domain_in_organisation
			)),
		],
	);

	// Get details of a domain
	app.get(
		"/:domainId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::VIEW_DETAILS,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::DOMAIN_ID).unwrap();
					let domain_id = hex::decode(&domain_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&domain_id,
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
			EveMiddleware::CustomFunction(pin_fn!(
				get_domain_info_in_organisation
			)),
		],
	);

	// Delete a domain
	app.delete(
		"/:domainId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::DELETE,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::DOMAIN_ID).unwrap();
					let domain_id = hex::decode(&domain_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&domain_id,
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
			EveMiddleware::CustomFunction(pin_fn!(
				delete_domain_in_organisation
			)),
		],
	);

	// Do something with the domains, etc, maybe?

	app
}

async fn get_domains_for_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let domains = db::get_domains_for_organisation(
		context.get_database_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|domain| {
		let id = domain.id.encode_hex::<String>();
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

async fn add_domain_to_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let organisation_id =
		hex::decode(&context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let body = context.get_body_object().clone();

	let domain_name = body
		.get(request_keys::DOMAIN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let domain_id = service::add_domain_to_organisation(
		context.get_database_connection(),
		&domain_name,
		&organisation_id,
	)
	.await?;
	let domain_id = domain_id.as_bytes().encode_hex::<String>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DOMAIN_ID: domain_id,
	}));
	Ok(context)
}

async fn verify_domain_in_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let domain_id_string =
		context.get_param(request_keys::DOMAIN_ID).unwrap().clone();

	// hex::decode throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless hex::decode(domain_id) returns Ok
	let domain_id = hex::decode(&domain_id_string).unwrap();

	let domain = db::get_organisation_domain_by_id(
		context.get_database_connection(),
		&domain_id,
	)
	.await?
	// Domain cannot be null.
	// This function wouldn't run unless the resource middleware
	// executes successfully The resource middleware checks if a
	// resource with that name exists. If the domain is null but the
	// resource exists, then you have a dangling resource. This is a big
	// problem. Make sure it's logged and investigated into
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let verified = service::is_domain_verified(
		context.get_database_connection(),
		&domain.id,
	)
	.await?;

	if verified {
		context.json(json!({
			request_keys::SUCCESS: true
		}));
	} else {
		// NOPE
		context.json(error!(DOMAIN_UNVERIFIED));
	}
	Ok(context)
}

async fn get_domain_info_in_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();

	// hex::decode throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless hex::decode(domain_id) returns Ok
	let domain_id = hex::decode(domain_id).unwrap();

	let domain = db::get_organisation_domain_by_id(
		context.get_database_connection(),
		&domain_id,
	)
	.await?;

	if domain.is_none() {
		// Domain cannot be null.
		// This function wouldn't run unless the resource middleware executes
		// successfully The resource middleware checks if a resource with that
		// name exists. If the domain is null but the resource exists, then you
		// have a dangling resource. This is a big problem. Make sure it's
		// logged and investigated into
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let domain = domain.unwrap();
	let domain_id = domain.id.encode_hex::<String>();

	context.json(
		if domain.is_verified {
			json!({
				request_keys::SUCCESS: true,
				request_keys::DOMAIN_ID: domain_id,
				request_keys::NAME: domain.name,
				request_keys::VERIFIED: true
			})
		} else {
			json!({
				request_keys::SUCCESS: true,
				request_keys::DOMAIN_ID: domain_id,
				request_keys::NAME: domain.name,
				request_keys::VERIFIED: false,
				request_keys::VERIFICATION_TOKEN: domain_id
			})
		},
	);
	Ok(context)
}

async fn delete_domain_in_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();

	// hex::decode throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless hex::decode(domain_id) returns Ok
	let domain_id = hex::decode(domain_id).unwrap();

	// TODO make sure all associated resources to this domain are removed first

	db::delete_domain_from_organisation(
		context.get_database_connection(),
		&domain_id,
	)
	.await?;
	db::delete_generic_domain(context.get_database_connection(), &domain_id)
		.await?;
	db::delete_resource(context.get_database_connection(), &domain_id).await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
