use api_macros::closure_as_pinned_box;
use api_models::{
	models::workspace::infrastructure::{
		managed_urls::{ManagedUrl, ManagedUrlType},
		static_site::{
			CreateStaticSiteRequest,
			CreateStaticSiteResponse,
			DeleteStaticSiteResponse,
			GetStaticSiteInfoResponse,
			ListLinkedURLsResponse,
			ListStaticSitesResponse,
			StartStaticSiteResponse,
			StaticSite,
			StaticSiteDetails,
			StopStaticSiteResponse,
			UpdateStaticSiteRequest,
			UpdateStaticSiteResponse,
		},
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db::{self, ManagedUrlType as DbManagedUrlType},
	error,
	models::{
		rbac::{permissions, PERMISSIONS},
		StaticSiteMetaData,
	},
	pin_fn,
	service,
	utils::{
		audit_logger::AuditLogData,
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

	// List all static sites
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::static_site::LIST,
				closure_as_pinned_box!(|mut context| {
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
			EveMiddleware::CustomFunction(pin_fn!(list_static_sites)),
		],
	);

	// Get info about a static sites
	app.get(
		"/:staticSiteId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::static_site::INFO,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = Uuid::parse_str(static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

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
				permissions::workspace::infrastructure::static_site::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = Uuid::parse_str(static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::AuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(start_static_site)),
		],
	);

	// Update static site
	app.patch(
		"/:staticSiteId/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::static_site::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = Uuid::parse_str(static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::AuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(update_static_site)),
		],
	);

	// stop the static site
	app.post(
		"/:staticSiteId/stop",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::static_site::EDIT,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = Uuid::parse_str(static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::AuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(stop_static_site)),
		],
	);

	// create static sites
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::static_site::CREATE,
				closure_as_pinned_box!(|mut context| {
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
			EveMiddleware::AuditLogger,
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
				permissions::workspace::infrastructure::static_site::DELETE,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = Uuid::parse_str(static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::AuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(delete_static_site)),
		],
	);

	// List all linked URLs of a static site
	app.get(
		"/:staticSiteId/managed-urls",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::infrastructure::static_site::INFO,
				closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let static_site_id_string = context
						.get_param(request_keys::STATIC_SITE_ID)
						.unwrap();
					let static_site_id = Uuid::parse_str(static_site_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&static_site_id,
					)
					.await?
					.filter(|value| value.owner_id == workspace_id);

					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_linked_urls)),
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
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();
	let static_site = db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	context.success(GetStaticSiteInfoResponse {
		static_site: StaticSite {
			id: static_site.id,
			name: static_site.name,
			status: static_site.status,
		},
		static_site_details: StaticSiteDetails {},
	});
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
	let request_id = context.get_request_id().clone();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	log::trace!("request_id: {} - Getting the list of all static sites for the workspace", request_id);
	let static_sites = db::get_static_sites_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|static_site| StaticSite {
		id: static_site.id,
		name: static_site.name,
		status: static_site.status,
	})
	.collect::<Vec<_>>();
	log::trace!("request_id: {} - Returning the list of all static sites for the workspace", request_id);

	context.success(ListStaticSitesResponse { static_sites });
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
	let request_id = context.get_request_id().clone();
	log::trace!("request_id: {} - Creating a static site", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let CreateStaticSiteRequest {
		workspace_id: _,
		name,
		file,
		static_site_details: _,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.trim();

	let config = context.get_state().config.clone();

	let id = service::create_static_site_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		name,
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	if let Some(file) = file {
		log::trace!("request_id: {} - Static-site created", request_id);
		log::trace!("request_id: {} - Starting the static site", request_id);
		service::queue_create_static_site(
			&workspace_id,
			&id,
			file,
			&config,
			&request_id,
		)
		.await?;
	}

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A static site has been created",
	)
	.await;

	context.set_audit_log_data(AuditLogData::WorkspaceResource {
		workspace_id,
		resource_id: id.clone(),
		action_id: PERMISSIONS
			.get()
			.and_then(|map| {
				map.get(
					permissions::workspace::infrastructure::static_site::CREATE,
				)
				.cloned()
			})
			.unwrap(),
		metadata: Some(serde_json::to_value(StaticSiteMetaData::Create {
			name: name.to_owned(),
		})?),
	});

	context.success(CreateStaticSiteResponse { id });
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
	let request_id = context.get_request_id().clone();
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();

	let static_site = db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - Starting a static site with id: {}",
		request_id,
		static_site_id
	);
	// start the container running the image, if doesn't exist
	let config = context.get_state().config.clone();
	service::queue_start_static_site(
		&static_site.workspace_id,
		&static_site_id,
		&config,
		&request_id,
	)
	.await?;

	context.set_audit_log_data(AuditLogData::WorkspaceResource {
		workspace_id: static_site.workspace_id,
		resource_id: static_site_id,
		action_id: PERMISSIONS
			.get()
			.and_then(|map| {
				map.get(
					permissions::workspace::infrastructure::static_site::EDIT,
				)
				.cloned()
			})
			.unwrap(),
		metadata: Some(serde_json::to_value(StaticSiteMetaData::Start {})?),
	});

	context.success(StartStaticSiteResponse {});
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
async fn update_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();
	let request_id = context.get_request_id().clone();
	log::trace!(
		"Uploading the file for static site with id: {} and request_id: {}",
		static_site_id,
		request_id
	);
	let UpdateStaticSiteRequest {
		workspace_id,
		static_site_id: _,
		name,
		file,
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.as_deref().map(|name| name.trim());

	let config = context.get_state().config.clone();

	let is_file_updated = file.is_some();
	service::update_static_site(
		context.get_database_connection(),
		&static_site_id,
		name,
		file,
		&config,
		&request_id,
	)
	.await?;

	context.set_audit_log_data(AuditLogData::WorkspaceResource {
		workspace_id,
		resource_id: static_site_id,
		action_id: PERMISSIONS
			.get()
			.and_then(|map| {
				map.get(
					permissions::workspace::infrastructure::static_site::EDIT,
				)
				.cloned()
			})
			.unwrap(),
		metadata: Some(serde_json::to_value(StaticSiteMetaData::Update {
			is_site_name_updated: name.is_some(),
			is_file_updated,
		})?),
	});

	context.success(UpdateStaticSiteResponse {});
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
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let request_id = context.get_request_id().clone();
	log::trace!(
		"request_id: {} - Stopping a static site with id: {}",
		request_id,
		static_site_id
	);

	// stop the running site, if it exists
	let config = context.get_state().config.clone();
	service::stop_static_site(
		context.get_database_connection(),
		&static_site_id,
		&config,
		&request_id,
	)
	.await?;

	context.set_audit_log_data(AuditLogData::WorkspaceResource {
		workspace_id,
		resource_id: static_site_id,
		action_id: PERMISSIONS
			.get()
			.and_then(|map| {
				map.get(
					permissions::workspace::infrastructure::static_site::EDIT,
				)
				.cloned()
			})
			.unwrap(),
		metadata: Some(serde_json::to_value(StaticSiteMetaData::Stop {})?),
	});

	context.success(StopStaticSiteResponse {});
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
	let request_id = context.get_request_id().clone();
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"request_id: {} - Deleting the static site with id: {}",
		request_id,
		static_site_id
	);

	// stop and delete the container running the image, if it exists
	let config = context.get_state().config.clone();
	service::delete_static_site(
		context.get_database_connection(),
		&static_site_id,
		&config,
		&request_id,
	)
	.await?;

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A static site has been deleted",
	)
	.await;

	context.set_audit_log_data(AuditLogData::WorkspaceResource {
		workspace_id,
		resource_id: static_site_id,
		action_id: PERMISSIONS
			.get()
			.and_then(|map| {
				map.get(
					permissions::workspace::infrastructure::static_site::DELETE,
				)
				.cloned()
			})
			.unwrap(),
		metadata: Some(serde_json::to_value(StaticSiteMetaData::Delete {})?),
	});

	context.success(DeleteStaticSiteResponse {});
	Ok(context)
}

async fn list_linked_urls(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = context.get_request_id().clone();
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();

	let workspace_id = Uuid::parse_str(
		context.get_param(request_keys::WORKSPACE_ID).unwrap(),
	)?;

	log::trace!(
		"request_id: {} - Listing the linked urls for static site with id: {}",
		request_id,
		static_site_id
	);
	let urls = db::get_all_managed_urls_for_static_site(
		context.get_database_connection(),
		&static_site_id,
		&workspace_id,
	)
	.await?
	.into_iter()
	.filter_map(|url| {
		Some(ManagedUrl {
			id: url.id,
			sub_domain: url.sub_domain,
			domain_id: url.domain_id,
			path: url.path,
			url_type: match url.url_type {
				DbManagedUrlType::ProxyToDeployment => {
					ManagedUrlType::ProxyDeployment {
						deployment_id: url.deployment_id?,
						port: url.port? as u16,
					}
				}
				DbManagedUrlType::ProxyToStaticSite => {
					ManagedUrlType::ProxyStaticSite {
						static_site_id: url.static_site_id?,
					}
				}
				DbManagedUrlType::ProxyUrl => {
					ManagedUrlType::ProxyUrl { url: url.url? }
				}
				DbManagedUrlType::Redirect => {
					ManagedUrlType::Redirect { url: url.url? }
				}
			},
		})
	})
	.collect();

	context.success(ListLinkedURLsResponse { urls });
	Ok(context)
}
