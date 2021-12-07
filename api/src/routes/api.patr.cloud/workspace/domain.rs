use cloudflare::{
	endpoints::{
		dns::{CreateDnsRecord, CreateDnsRecordParams},
		zone::{
			CreateZone,
			CreateZoneParams,
			ListZones,
			ListZonesParams,
			Type,
		},
	},
	framework::{
		async_api::{ApiClient, Client as CloudflareClient},
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
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

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut app = create_eve_app(app);

	// Get all domains
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::domain::LIST,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = hex::decode(&workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_domains_for_workspace)),
		],
	);

	// Add a domain
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::domain::ADD,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = hex::decode(&workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
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
			EveMiddleware::CustomFunction(pin_fn!(add_domain_to_workspace)),
		],
	);

	// Verify a domain
	app.post(
		"/:domainId/verify",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::domain::VERIFY,
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
			EveMiddleware::CustomFunction(pin_fn!(verify_domain_in_workspace)),
		],
	);

	// Get details of a domain
	app.get(
		"/:domainId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::domain::VIEW_DETAILS,
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
				get_domain_info_in_workspace
			)),
		],
	);

	// Delete a domain
	app.delete(
		"/:domainId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::domain::DELETE,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_domain_in_workspace)),
		],
	);

	app.post(
		"/:deploymentId/automate-dns-control",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::domain::ADD,
				api_macros::closure_as_pinned_box!(|mut context| {
					let deployment_id_string =
						context.get_param(request_keys::DEPLOYMENT_ID).unwrap();
					let deployment_id = hex::decode(&deployment_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&deployment_id,
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
			EveMiddleware::CustomFunction(pin_fn!(add_domain_and_create_zone)),
		],
	);
	// Do something with the domains, etc, maybe?

	app
}

/// # Description
/// This function is used to get the list of domains present under the
/// workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    domains:
///    [
///       {
///          id:
///          name:
///          verified:
///       }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_domains_for_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		hex::decode(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let domains = db::get_domains_for_workspace(
		context.get_database_connection(),
		&workspace_id,
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

/// # Description
/// This function is used to add a domain to an workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in url
/// ```
/// {
///     domain:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    domainId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn add_domain_to_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		hex::decode(&context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let body = context.get_body_object().clone();

	let domain_name = body
		.get(request_keys::DOMAIN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let domain_id = service::add_domain_to_workspace(
		context.get_database_connection(),
		&domain_name,
		&workspace_id,
	)
	.await?;
	let domain_id = domain_id.as_bytes().encode_hex::<String>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DOMAIN_ID: domain_id,
	}));
	Ok(context)
}

/// # Description
/// This function is used to verify a domain which is to be registered under an
/// workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in the url
/// ```
/// {
///     domainId:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn verify_domain_in_workspace(
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

	let domain = db::get_workspace_domain_by_id(
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

/// # Description
/// This function is used to get details about a domain registered under the
/// workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in the url
/// ```
/// {
///     domainId:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false,
///    domainId: ,
///    name: ,
///    verified: true
/// }
/// if not verified
/// {
///    success: true or false,
///    domainId: ,
///    name: ,
///    verified: true,
///    verificationToken:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_domain_info_in_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();

	// hex::decode throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless hex::decode(domain_id) returns Ok
	let domain_id = hex::decode(domain_id).unwrap();

	let domain = db::get_workspace_domain_by_id(
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

/// # Description
/// This function is used to delete the domain registered under the workspace
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// workspace id in the url
/// ```
/// {
///    domainId:
/// }
/// ```
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error output:
/// ```
/// {
///    success: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn delete_domain_in_workspace(
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

	db::delete_domain_from_workspace(
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

/// #Description
/// This function registers given domain on Patr and Cloudflare account and
/// provides the user with domain id
///
/// TODO: change the name of this function to something more appropriate and
/// also implement adding domain to the database here. return Domain id upon
/// successfull zone creation on CloudFlare
async fn add_domain_and_create_zone(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		hex::decode(&context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	let domain_name = body
		.get(request_keys::DOMAIN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	let domain_id = service::add_domain_to_workspace(
		context.get_database_connection(),
		&domain_name,
		&workspace_id,
	)
	.await?;

	let config = context.get_state().config.clone();

	// login to cloudflare and create zone in cloudflare
	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};

	let client = if let Ok(client) = CloudflareClient::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	) {
		client
	} else {
		return Err(Error::empty());
	};

	// create zone
	client
		.request(&CreateZone {
			params: CreateZoneParams {
				name: &domain_name,
				jump_start: Some(false),
				account: &config.cloudflare.account_id,
				// Full because the DNS record
				zone_type: Some(Type::Full),
			},
		})
		.await?;

	let zone_identifier = client
		.request(&ListZones {
			params: ListZonesParams {
				name: Some(domain_name),
				..Default::default()
			},
		})
		.await?
		.result
		.into_iter()
		.next()
		.status(500)?
		.id;
	let _zone_identifier = zone_identifier.as_str();
	// todo: store the zone id in database

	// TODO: update DNS A record
	// let update_dns_record_status = client.request( &CreateDnsRecord{
	// 	zone_identifier,
	// 	params: CreateDnsRecordParams{
	// 		ttl: Some(1),
	// 		priority: None,
	// 		proxied: None,
	// 		name: &domain_name,
	// 		content: &domain_name,
	// })
	// .await?;

	// TODO: send nameserver details to the user
	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DOMAIN_ID: domain_id.to_simple().to_string(),
	}));

	Ok(context)
}
