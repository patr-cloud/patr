use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{constants::request_keys, EveContext, EveMiddleware},
};

use argon2::Variant;
use eve_rs::{App as EveApp, Context, Error, NextHandler};
use serde_json::{json, Value};
use trust_dns_client::{
	client::{Client, SyncClient},
	rr::{DNSClass, Name, RData, RecordType},
	tcp::TcpClientConnection,
};

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(&app);

	// Get all domains
	app.get(
		"/",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::LIST,
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
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::ADD,
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
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::VERIFY,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::DOMAIN_ID).unwrap();
					let domain_id = hex::decode(&domain_id_string);
					if domain_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let domain_id = domain_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
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
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::VIEW_DETAILS,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::DOMAIN_ID).unwrap();
					let domain_id = hex::decode(&domain_id_string);
					if domain_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let domain_id = domain_id.unwrap();
					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
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
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::domain::DELETE,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::DOMAIN_ID).unwrap();
					let domain_id = hex::decode(&domain_id_string);
					if domain_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let domain_id = domain_id.unwrap();
					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
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
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let organisation_id =
		hex::decode(context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let domains = db::get_domains_for_organisation(
		context.get_mysql_connection(),
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

async fn add_domain_to_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let organisation_id =
		hex::decode(&context.get_param(request_keys::ORGANISATION_ID).unwrap())
			.unwrap();

	let body = if let Value::Object(body) = context.get_body_object() {
		body.clone()
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let domain_name =
		if let Some(Value::String(domain)) = body.get(request_keys::DOMAIN) {
			domain
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let domain_exists = db::get_domains_for_organisation(
		context.get_mysql_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.any(|domain| &domain.name == domain_name);

	if domain_exists {
		context.status(400).json(error!(RESOURCE_EXISTS));
		return Ok(context);
	}

	let domain_id =
		db::generate_new_resource_id(context.get_mysql_connection()).await?;
	let domain_id = domain_id.as_bytes();

	db::create_resource(
		context.get_mysql_connection(),
		domain_id,
		&format!("Domain: {}", domain_name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::DOMAIN)
			.unwrap(),
		&organisation_id,
	)
	.await?;
	db::add_domain_to_organisation(
		context.get_mysql_connection(),
		domain_id,
		domain_name,
	)
	.await?;

	let domain_id = hex::encode(domain_id);
	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DOMAIN_ID: domain_id,
	}));
	Ok(context)
}

async fn verify_domain_in_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();

	// hex::decode throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless hex::decode(domain_id) returns Ok
	let domain_id = hex::decode(domain_id).unwrap();

	let domain =
		db::get_domain_by_id(context.get_mysql_connection(), &domain_id)
			.await?;

	if domain.is_none() {
		// Domain cannot be null.
		// This function wouldn't run unless the resource middleware executes successfully
		// The resource middleware checks if a resource with that name exists.
		// If the domain is null but the resource exists, then you have a dangling resource.
		// This is a big problem. Make sure it's logged and investigated into
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let domain = domain.unwrap();

	let domain_hash = argon2::hash_raw(
		domain.name.as_bytes(),
		context.get_state().config.password_salt.as_bytes(),
		&argon2::Config {
			variant: Variant::Argon2i,
			hash_length: 64,
			..Default::default()
		},
	)?;
	let domain_hash = hex::encode(domain_hash);

	let client = SyncClient::new(
		TcpClientConnection::new("1.1.1.1:53".parse().unwrap()).unwrap(),
	);
	let mut response = client.query(
		&Name::from_utf8(format!("vceVerify.{}", domain.name)).unwrap(),
		DNSClass::IN,
		RecordType::CNAME,
	)?;
	let response = response.take_answers().into_iter().find(|record| {
		let expected_cname = RData::CNAME(
			Name::from_utf8(format!("{}.vicara.co", domain_hash)).unwrap(),
		);
		record.rdata() == &expected_cname
	});

	if response.is_some() {
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
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();

	// hex::decode throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless hex::decode(domain_id) returns Ok
	let domain_id = hex::decode(domain_id).unwrap();

	let domain =
		db::get_domain_by_id(context.get_mysql_connection(), &domain_id)
			.await?;

	if domain.is_none() {
		// Domain cannot be null.
		// This function wouldn't run unless the resource middleware executes successfully
		// The resource middleware checks if a resource with that name exists.
		// If the domain is null but the resource exists, then you have a dangling resource.
		// This is a big problem. Make sure it's logged and investigated into
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let domain = domain.unwrap();
	let domain_id = hex::encode(domain.id);

	context.json(
		if domain.is_verified {
			json!({
				request_keys::SUCCESS: true,
				request_keys::DOMAIN_ID: domain_id,
				request_keys::NAME: domain.name,
				request_keys::VERIFIED: true
			})
		} else {
			let domain_hash = argon2::hash_raw(
				domain.name.as_bytes(),
				context.get_state().config.password_salt.as_bytes(),
				&argon2::Config {
					variant: Variant::Argon2i,
					hash_length: 64,
					..Default::default()
				},
			)?;
			let domain_hash = hex::encode(domain_hash);

			json!({
				request_keys::SUCCESS: true,
				request_keys::DOMAIN_ID: domain_id,
				request_keys::NAME: domain.name,
				request_keys::VERIFIED: false,
				request_keys::VERIFICATION_TOKEN: domain_hash
			})
		},
	);
	Ok(context)
}

async fn delete_domain_in_organisation(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();

	// hex::decode throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless hex::decode(domain_id) returns Ok
	let domain_id = hex::decode(domain_id).unwrap();

	// TODO make sure all associated resources to this domain are removed first

	db::delete_domain_from_organisation(
		context.get_mysql_connection(),
		&domain_id,
	)
	.await?;
	db::delete_resource(context.get_mysql_connection(), &domain_id).await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}
