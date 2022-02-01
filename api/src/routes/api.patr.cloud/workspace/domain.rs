use api_models::{
	models::workspace::domain::{
		AddDnsRecordRequest,
		AddDnsRecordResponse,
		AddDomainRequest,
		AddDomainResponse,
		DeleteDnsRecordResponse,
		DeleteDomainResponse,
		DnsRecordValue,
		Domain,
		GetDomainDnsRecordsResponse,
		GetDomainInfoResponse,
		GetDomainsForWorkspaceResponse,
		PatrDomainDnsRecord,
		UpdateDomainDnsRecordRequest,
		UpdateDomainDnsRecordResponse,
		VerifyDomainResponse,
		WorkspaceDomain,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{db_mapping::DnsRecordType, rbac::permissions},
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
				permissions::workspace::domain::dns_record::LIST,
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
				permissions::workspace::domain::dns_record::ADD,
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
				permissions::workspace::domain::dns_record::EDIT,
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
				permissions::workspace::domain::dns_record::DELETE,
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
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Getting domains for workspace", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let domains = db::get_domains_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|domain| WorkspaceDomain {
		domain: Domain {
			id: domain.id,
			name: domain.name,
		},
		is_verified: domain.is_verified,
		nameserver_type: domain.nameserver_type,
	})
	.collect();

	log::trace!(
		"request_id: {} - Returning domains for workspace",
		request_id
	);
	context.success(GetDomainsForWorkspaceResponse { domains });
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
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Adding domain to workspace", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let AddDomainRequest {
		workspace_id: _,
		domain,
		nameserver_type,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// move this to service layer
	let config = context.get_state().config.clone();
	let domain_id = service::add_domain_to_workspace(
		context.get_database_connection(),
		&domain,
		&nameserver_type,
		&workspace_id,
		&config,
		&request_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - Added the domain to the workspace",
		request_id
	);
	context.success(AddDomainResponse { id: domain_id });

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
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Verifying domain in workspace", request_id);
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)?;

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
		&workspace_id,
		&config,
		&request_id,
	)
	.await?;

	context.success(VerifyDomainResponse { verified });
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
	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Getting domain info in workspace",
		request_id
	);
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
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	context.success(GetDomainInfoResponse {
		domain: Domain {
			id: domain.id,
			name: domain.name,
		},
		is_verified: domain.is_verified,
	});

	log::trace!("request_id: {} - Got domain info in workspace", request_id);
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
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Deleting domain in workspace", request_id);
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)?;

	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();
	// Uuid::parse_str throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless Uuid::parse_str(domain_id) returns Ok
	let domain_id = Uuid::parse_str(domain_id).unwrap();

	// TODO make sure all associated resources to this domain are removed first

	let config = context.get_state().config.clone();

	service::delete_domain_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		&domain_id,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Deleted domain in workspace", request_id);
	// TODO: add the info to patr metrics
	context.success(DeleteDomainResponse {});
	Ok(context)
}

async fn get_domain_dns_record(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Getting domain dns record", request_id);
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();

	// Uuid::parse_str throws an error for a wrong string
	// This error is handled by the resource authenticator middleware
	// So it's safe to call unwrap() here without crashing the system
	// This won't be executed unless Uuid::parse_str(domain_id) returns Ok
	let domain_id = Uuid::parse_str(domain_id).unwrap();

	// get dns records from database
	let records = db::get_dns_records_by_domain_id(
		context.get_database_connection(),
		&domain_id,
	)
	.await?
	.into_iter()
	.filter_map(|record| {
		let proxied = if let Some(proxied) = record.proxied {
			proxied
		} else {
			false
		};
		let record_value = match record.r#type {
			DnsRecordType::A => DnsRecordValue::A {
				target: record.value,
				proxied,
			},
			DnsRecordType::AAAA => DnsRecordValue::AAAA {
				target: record.value,
				proxied,
			},
			DnsRecordType::CNAME => DnsRecordValue::CNAME {
				target: record.value,
				proxied,
			},
			DnsRecordType::MX => DnsRecordValue::MX {
				target: record.value,
				priority: record.priority.map(|p| p as u16)?,
			},
			DnsRecordType::TXT => DnsRecordValue::TXT {
				target: record.value,
			},
		};
		Some(PatrDomainDnsRecord {
			id: record.id,
			domain_id: record.domain_id,
			name: record.name,
			r#type: record_value,
			ttl: record.ttl as u32,
		})
	})
	.collect();

	log::trace!("request_id: {} - Got domain dns record", request_id);
	context.success(GetDomainDnsRecordsResponse { records });
	Ok(context)
}

async fn add_dns_record(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Adding dns record", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	let domain_id =
		Uuid::parse_str(context.get_param(request_keys::DOMAIN_ID).unwrap())
			.status(400)?;

	let AddDnsRecordRequest {
		workspace_id: _,
		domain_id: _,
		name,
		r#type,
		ttl,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	// add a record to cloudflare
	let record_id = service::create_patr_domain_dns_record(
		context.get_database_connection(),
		&workspace_id,
		&domain_id,
		&name,
		ttl,
		&r#type,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Added dns record", request_id);
	context.success(AddDnsRecordResponse { id: record_id });
	Ok(context)
}

async fn update_dns_record(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Updating dns record", request_id);
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();
	let domain_id = Uuid::parse_str(domain_id)?;

	let record_id = context.get_param(request_keys::RECORD_ID).unwrap();
	let record_id = Uuid::parse_str(record_id)?;

	let UpdateDomainDnsRecordRequest {
		workspace_id: _,
		domain_id: _,
		record_id: _,
		ttl,
		proxied,
		target,
		priority,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	service::update_patr_domain_dns_record(
		context.get_database_connection(),
		&domain_id,
		&record_id,
		target.as_deref(),
		ttl,
		proxied,
		priority,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Updated dns record", request_id);
	context.success(UpdateDomainDnsRecordResponse {});
	Ok(context)
}

async fn delete_dns_record(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	log::trace!("request_id: {} - Deleting dns record", request_id);
	let domain_id = context.get_param(request_keys::DOMAIN_ID).unwrap();
	let domain_id = Uuid::parse_str(domain_id)?;

	let record_id = context.get_param(request_keys::RECORD_ID).unwrap();
	let record_id = Uuid::parse_str(record_id)?;

	let config = context.get_state().config.clone();

	service::delete_patr_domain_dns_record(
		context.get_database_connection(),
		&domain_id,
		&record_id,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Deleted dns record", request_id);
	context.success(DeleteDnsRecordResponse {});
	Ok(context)
}
