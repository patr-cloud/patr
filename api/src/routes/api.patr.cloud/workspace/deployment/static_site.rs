use api_macros::closure_as_pinned_box;
use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	pin_fn,
	service,
	utils::{
		constants::request_keys,
		validator,
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

	// List all static sites
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::LIST,
				closure_as_pinned_box!(|mut context| {
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
			EveMiddleware::CustomFunction(pin_fn!(list_static_sites)),
		],
	);

	// Get info about a static sites
	app.get(
		"/:staticSiteId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::INFO,
				closure_as_pinned_box!(|mut context| {
					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = hex::decode(&static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_static_site_info)),
		],
	);

	// start a static site
	app.post(
		"/:staticSiteId/start",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::EDIT,
				closure_as_pinned_box!(|mut context| {
					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = hex::decode(&static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
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
			EveMiddleware::CustomFunction(pin_fn!(start_static_site)),
		],
	);

	// Upload a new site to the static site
	app.put(
		"/:staticSiteId/upload",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::EDIT,
				closure_as_pinned_box!(|mut context| {
					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = hex::decode(&static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
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
				upload_files_for_static_site
			)),
		],
	);

	// stop the static site
	app.post(
		"/:staticSiteId/stop",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::EDIT,
				closure_as_pinned_box!(|mut context| {
					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = hex::decode(&static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
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
			EveMiddleware::CustomFunction(pin_fn!(stop_static_site)),
		],
	);

	// create static sites
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::CREATE,
				closure_as_pinned_box!(|mut context| {
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
			EveMiddleware::CustomFunction(pin_fn!(
				create_static_site_deployment
			)),
		],
	);

	// Delete a static_site
	app.delete(
		"/:staticSiteId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::DELETE,
				closure_as_pinned_box!(|mut context| {
					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = hex::decode(&static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_static_site)),
		],
	);

	// get domain cname and value of static_site
	app.get(
		"/:staticSiteId/domain-dns-records",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::INFO,
				closure_as_pinned_box!(|mut context| {
					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = hex::decode(&static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
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
				get_domain_dns_records_for_static_site
			)),
		],
	);

	// update domain in the static_site
	app.put(
		"/:staticSiteId/domain",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::EDIT,
				closure_as_pinned_box!(|mut context| {
					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = hex::decode(&static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
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
				set_domain_name_for_static_site
			)),
		],
	);

	// get static_site validation status
	app.get(
		"/:staticSiteId/domain-validated",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::static_site::INFO,
				closure_as_pinned_box!(|mut context| {
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
			EveMiddleware::CustomFunction(pin_fn!(
				is_domain_validated_for_static_site
			)),
		],
	);

	app
}

/// # Description
/// This function is used to get the information about a specific static_site
/// required inputs:
/// auth token in the authorization headers
/// example: Authorization: <insert authToken>
/// staticSiteId in url
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
///     success: true or false,
///     staticSites:
///     {
///         id: ,
///         name: ,
///         domainName: ,
///     }
/// }
/// ```
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_static_site_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id =
		hex::decode(context.get_param(request_keys::STATIC_SITE_ID).unwrap())
			.unwrap();
	let static_site = db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let mut response = Map::new();

	response.insert(request_keys::SUCCESS.to_string(), Value::Bool(true));

	response.insert(
		request_keys::STATIC_SITE_ID.to_string(),
		Value::String(hex::encode(static_site.id)),
	);
	response.insert(
		request_keys::NAME.to_string(),
		Value::String(static_site.name),
	);
	response.insert(
		request_keys::STATUS.to_string(),
		Value::String(static_site.status.to_string()),
	);
	if let Some(domain_name) = static_site.domain_name {
		response.insert(
			request_keys::DOMAIN_NAME.to_string(),
			Value::String(domain_name),
		);
	}

	context.json(Value::Object(response));
	Ok(context)
}

/// # Description
/// This function is used to list of all the static sites present with the user
/// required inputs:
/// WorkspaceId in url
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
///    success:
///    staticSites: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_static_sites(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		hex::decode(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let static_sites = db::get_static_sites_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|static_site| {
		let mut map = Map::new();

		map.insert(request_keys::SUCCESS.to_string(), Value::Bool(true));
		map.insert(
			request_keys::NAME.to_string(),
			Value::String(static_site.name),
		);
		map.insert(
			request_keys::STATIC_SITE_ID.to_string(),
			Value::String(hex::encode(static_site.id)),
		);
		map.insert(
			request_keys::STATUS.to_string(),
			Value::String(static_site.status.to_string()),
		);
		if let Some(domain_name) = static_site.domain_name {
			map.insert(
				request_keys::DOMAIN_NAME.to_string(),
				Value::String(domain_name),
			);
		}
		Some(Value::Object(map))
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::STATIC_SITES: static_sites
	}));
	Ok(context)
}

/// # Description
/// This function is used to create a new static site
/// required inputs
/// auth token in the header
/// workspace id in parameter
/// ```
/// {
///    name: ,
///    domainName:
/// }
/// ```
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
///    success:
///    staticSiteId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create_static_site_deployment(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		hex::decode(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	let name = body
		.get(request_keys::NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?
		.trim();

	let domain_name = body
		.get(request_keys::DOMAIN_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let file = body
		.get(request_keys::STATIC_SITE_FILE)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	let config = context.get_state().config.clone();

	let static_site_id = service::create_static_site_deployment_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		name,
		domain_name,
	)
	.await?;

	context.commit_database_transaction().await?;

	service::start_static_site_deployment(
		context.get_database_connection(),
		static_site_id.as_bytes(),
		&config,
		file,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::STATIC_SITE_ID: hex::encode(static_site_id.as_bytes())
	}));

	Ok(context)
}

/// # Description
/// This function is used to start a static site
/// required inputs:
/// staticSiteId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
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
async fn start_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id =
		hex::decode(context.get_param(request_keys::STATIC_SITE_ID).unwrap())
			.unwrap();

	// start the container running the image, if doesn't exist
	let config = context.get_state().config.clone();
	service::start_static_site_deployment(
		context.get_database_connection(),
		&static_site_id,
		&config,
		None,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the status of domain set for static site
/// required inputs:
/// staticSiteId in the url
/// ```
/// {
///     domainName:
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
async fn upload_files_for_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id =
		hex::decode(context.get_param(request_keys::STATIC_SITE_ID).unwrap())
			.unwrap();
	let body = context.get_body_object().clone();

	let file = body
		.get(request_keys::STATIC_SITE_FILE)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();
	let request_id = Uuid::new_v4();
	log::trace!(
		"Uploading the file for static site with id: {} and request_id: {}",
		hex::encode(&static_site_id),
		request_id
	);
	service::upload_files_for_static_site(
		context.get_database_connection(),
		&static_site_id,
		file,
		&config,
		request_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to stop a static site
/// required inputs:
/// staticSiteId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
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
async fn stop_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id =
		hex::decode(context.get_param(request_keys::STATIC_SITE_ID).unwrap())
			.unwrap();

	// stop the running site, if it exists
	let config = context.get_state().config.clone();
	service::stop_static_site(
		context.get_database_connection(),
		&static_site_id,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to stop a static site
/// required inputs:
/// staticSiteId in the url
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the next
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
async fn delete_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id =
		hex::decode(context.get_param(request_keys::STATIC_SITE_ID).unwrap())
			.unwrap();

	// stop and delete the container running the image, if it exists
	let config = context.get_state().config.clone();
	service::delete_static_site(
		context.get_database_connection(),
		&static_site_id,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the DNS records for the static site
/// required inputs:
/// staticSiteId in the url
/// ```
/// {
///     domainName:
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
///    cnameRecords: [
///         {
///           cname: "domain_name",
///           value: "provider's url"
///         }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_domain_dns_records_for_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id =
		hex::decode(context.get_param(request_keys::STATIC_SITE_ID).unwrap())
			.unwrap();

	let config = context.get_state().config.clone();

	let cname_records = service::get_dns_records_for_static_site(
		context.get_database_connection(),
		&static_site_id,
		config,
	)
	.await?
	.into_iter()
	.map(|record| {
		json!({
			request_keys::CNAME: record.cname,
			request_keys::VALUE: record.value
		})
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::CNAME_RECORDS: cname_records
	}));
	Ok(context)
}

/// # Description
/// This function is used to set the domain name of the static site
/// required inputs:
/// staticSiteId in the url
/// ```
/// {
///     domainName:
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
///    cnameRecords: [
///         {
///           cname: "domain_name",
///           value: "provider's url"
///         }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn set_domain_name_for_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id =
		hex::decode(context.get_param(request_keys::STATIC_SITE_ID).unwrap())
			.unwrap();

	let body = context.get_body_object().clone();
	let domain_name = body
		.get(request_keys::DOMAIN_NAME)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;

	if let Some(domain_name) = domain_name {
		if !validator::is_deployment_entry_point_valid(domain_name) {
			return Err(Error::empty()
				.status(400)
				.body(error!(INVALID_DOMAIN_NAME).to_string()));
		}
	}
	let config = context.get_state().config.clone();

	service::set_domain_for_static_site_deployment(
		context.get_database_connection(),
		&config,
		&static_site_id,
		domain_name,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the status of domain set for static site
/// required inputs:
/// staticSiteId in the url
/// ```
/// {
///     domainName:
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
async fn is_domain_validated_for_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id =
		hex::decode(context.get_param(request_keys::STATIC_SITE_ID).unwrap())
			.unwrap();
	let config = context.get_state().config.clone();

	let validated = service::get_static_site_validation_status(
		context.get_database_connection(),
		&static_site_id,
		&config,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::VALIDATED: validated,
	}));
	Ok(context)
}
