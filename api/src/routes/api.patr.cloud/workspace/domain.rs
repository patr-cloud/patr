use api_models::utils::Uuid;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::json;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		db_mapping::{DnsRecordType, DomainNameserverType},
		rbac::permissions,
	},
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
					let workspace_id = Uuid::parse_str(workspace_id_string)
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
					let workspace_id = Uuid::parse_str(workspace_id_string)
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
					let domain_id = Uuid::parse_str(domain_id_string)
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
					let domain_id = Uuid::parse_str(domain_id_string)
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
					let domain_id = Uuid::parse_str(domain_id_string)
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

	// Get list of records for a domain
	app.get(
		"/:domainId/dns-record",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::dns_record::LIST,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::DOMAIN_ID).unwrap();
					let domain_id = Uuid::parse_str(domain_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(get_domain_dns_record)),
		],
	);

	//  add dns record
	app.post(
		"/:domainId/dns-record",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::dns_record::ADD,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::DOMAIN_ID).unwrap();
					let domain_id = Uuid::parse_str(domain_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(add_dns_record)),
		],
	);

	// update dns record
	app.patch(
		"/:domainId/dns-record/:recordId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::dns_record::EDIT,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::RECORD_ID).unwrap();
					let domain_id = Uuid::parse_str(domain_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(update_dns_record)),
		],
	);

	app.delete(
		"/:domainId/dns-record/:recordId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::dns_record::DELETE,
				api_macros::closure_as_pinned_box!(|mut context| {
					let domain_id_string =
						context.get_param(request_keys::RECORD_ID).unwrap();
					let domain_id = Uuid::parse_str(domain_id_string)
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
			EveMiddleware::CustomFunction(pin_fn!(delete_dns_record)),
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
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let domains = db::get_domains_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|domain| {
		json!({
			request_keys::ID: domain.id,
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
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let body = context.get_body_object().clone();

	let domain_name = body
		.get(request_keys::DOMAIN)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.to_lowercase();

	// will determine if we control the DNS records or the user
	let nameserver_type = body
		.get(request_keys::DOMAIN_NAMESERVER_TYPE)
		.map(|value| value.as_str())
		.flatten()
		.map(|value| value.parse::<DomainNameserverType>().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// move this to service layer
	let config = context.get_state().config.clone();
	let domain_id = service::add_domain_to_workspace(
		context.get_database_connection(),
		&config,
		&domain_name,
		&workspace_id,
		&nameserver_type,
	)
	.await?;

	if let DomainNameserverType::Internal = nameserver_type {
		context.json(json!({
			request_keys::SUCCESS: true,
			request_keys::DOMAIN_ID: domain_id,
		}));
	} else {
		context.json(json!({
			request_keys::SUCCESS: true,
			request_keys::DOMAIN_ID: domain_id,
			request_keys::TXT_RECORD: {
				request_keys::TARGET: format!("patrVerify.{}",domain_name),
				request_keys::CONTENT: domain_id,
			}
		}));
	}

	Ok(context)
}

/// # Description
/// This function is used to verify a domain which is to be registered under a
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

	// Uuid::parse_str throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless Uuid::parse_str(domain_id) returns Ok
	let domain_id = Uuid::parse_str(&domain_id_string).unwrap();

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

	let config = context.get_state().config.clone();

	let verified = service::is_domain_verified(
		context.get_database_connection(),
		&domain.id,
		&config,
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

	// Uuid::parse_str throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless Uuid::parse_str(domain_id) returns Ok
	let domain_id = Uuid::parse_str(domain_id).unwrap();

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

	context.json(
		if domain.is_verified {
			json!({
				request_keys::SUCCESS: true,
				request_keys::DOMAIN_ID: domain.id,
				request_keys::NAME: domain.name,
				request_keys::VERIFIED: true
			})
		} else {
			json!({
				request_keys::SUCCESS: true,
				request_keys::DOMAIN_ID: domain.id,
				request_keys::NAME: domain.name,
				request_keys::VERIFIED: false,
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

	// Uuid::parse_str throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless Uuid::parse_str(domain_id) returns Ok
	let domain_id = Uuid::parse_str(domain_id).unwrap();

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

async fn get_domain_dns_record(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();

	// Uuid::parse_str throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless Uuid::parse_str(domain_id) returns Ok
	let domain_id = Uuid::parse_str(domain_id).unwrap();

	// get dns records from database
	let dns_record = db::get_dns_records_by_domain_id(
		context.get_database_connection(),
		&domain_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::DNS_RECORD: dns_record
	}));
	Ok(context)
}

// NOTE: this function can be used for both adding and updating dns records
async fn add_dns_record(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// TODO: convert the second unwrap to ?
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let domain_id =
		Uuid::parse_str(context.get_param(request_keys::DOMAIN_ID).unwrap())
			.unwrap();

	// check if domain is patr controlled
	// TODO: maybe change the error code to something more appropriate
	let domain = db::get_patr_controlled_domain_by_id(
		context.get_database_connection(),
		&domain_id,
	)
	.await?
	.status(400)
	.body(error!(DOMAIN_NOT_PATR_CONTROLLED).to_string())?;

	let body = context.get_body_object().clone();

	// type determines what kind of record is being added
	let r#type = body
		.get(request_keys::TYPE)
		.map(|value| value.as_str())
		.flatten()
		.map(|value| value.parse::<DnsRecordType>().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let name = body
		.get(request_keys::NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let ttl = body
		.get(request_keys::TTL)
		.map(|value| value.as_u64())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let proxied = body
		.get(request_keys::PROXIED)
		.map(|value| value.as_bool())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	let record = body
		.get(request_keys::CONTENT)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// add a record to cloudflare
	let record_id = service::add_patr_dns_record(
		context.get_database_connection(),
		&workspace_id,
		&domain_id,
		&domain.zone_identifier,
		name,
		record,
		ttl as u32,
		proxied,
		if let DnsRecordType::MX = &r#type {
			Some(
				body.get(request_keys::PRIORITY)
					.map(|value| value.as_u64())
					.flatten()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string())?,
			)
		} else {
			None
		},
		&r#type,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::RECORD_ID: hex::encode(record_id.as_bytes()),
	}));
	Ok(context)
}

async fn update_dns_record(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();
	let domain_id = Uuid::parse_str(domain_id).unwrap();

	let record_id = context.get_param(request_keys::RECORD_ID).unwrap();
	let record_id = Uuid::parse_str(record_id).unwrap();

	db::get_dns_record_by_id(context.get_database_connection(), &record_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let domain = db::get_patr_controlled_domain_by_id(
		context.get_database_connection(),
		&domain_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let body = context.get_body_object().clone();

	let ttl = body
		.get(request_keys::TTL)
		.map(|value| {
			value
				.as_u64()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let r#type = body
		.get(request_keys::TYPE)
		.map(|value| value.as_str())
		.flatten()
		.map(|value| value.parse::<DnsRecordType>().ok())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let proxied = body
		.get(request_keys::PROXIED)
		.map(|value| {
			value
				.as_bool()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let record = body
		.get(request_keys::CONTENT)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let priority = body
		.get(request_keys::PRIORITY)
		.map(|value| {
			value
				.as_u64()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let config = context.get_state().config.clone();

	service::update_patr_dns_record(
		context.get_database_connection(),
		&record_id,
		&domain.zone_identifier,
		record,
		ttl.map(|value| value as u32),
		proxied,
		priority.map(|value| value as u16),
		&r#type,
		&config,
	)
	.await?;

	Ok(context)
}

async fn delete_dns_record(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();
	let domain_id = Uuid::parse_str(domain_id).unwrap();

	let record_id = context.get_param(request_keys::RECORD_ID).unwrap();
	let record_id = Uuid::parse_str(record_id).unwrap();

	// check if domain is patr controlled
	db::get_patr_controlled_domain_by_id(
		context.get_database_connection(),
		&domain_id,
	)
	.await?
	.status(400)
	.body(error!(DOMAIN_NOT_PATR_CONTROLLED).to_string())?;

	db::get_dns_record_by_id(context.get_database_connection(), &record_id)
		.await?
		.status(400)
		.body(error!(DNS_RECORD_NOT_FOUND).to_string())?;

	db::delete_patr_controlled_dns_record(
		context.get_database_connection(),
		&record_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
	}));
	Ok(context)
}
