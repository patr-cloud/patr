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
			ListStaticSiteUploadHistoryResponse,
			ListStaticSitesResponse,
			RevertStaticSiteResponse,
			StartStaticSiteResponse,
			StaticSite,
			StaticSiteDetails,
			StaticSiteUploadHistory,
			StopStaticSiteResponse,
			UpdateStaticSiteRequest,
			UpdateStaticSiteResponse,
			UploadStaticSiteRequest,
			UploadStaticSiteResponse,
		},
	},
	utils::{DateTime, Uuid},
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db::{self, ManagedUrlType as DbManagedUrlType},
	error,
	models::{rbac::permissions, ResourceType},
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

	// List all static sites
	app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::LIST,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(list_static_sites)),
		],
	);

	// List all uploads for static site
	app.get(
		"/:staticSiteId/upload",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::LIST,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(
				list_static_sites_upload_history
			)),
		],
	);

	// Get info about a static sites
	app.get(
		"/:staticSiteId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(get_static_site_info)),
		],
	);

	// start a static site
	app.post(
		"/:staticSiteId/start",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(start_static_site)),
		],
	);

	// Update static site
	app.patch(
		"/:staticSiteId/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(update_static_site)),
		],
	);

	// Upload static site
	app.post(
		"/:staticSiteId/upload",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(upload_static_site)),
		],
	);

	// Revert static site
	app.post(
		"/:staticSiteId/upload/:uploadId/revert",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(revert_static_site)),
		],
	);

	// stop the static site
	app.post(
		"/:staticSiteId/stop",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::EDIT,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(stop_static_site)),
		],
	);

	// create static sites
	app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::CREATE,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(create_static_site)),
		],
	);

	// Delete a static_site
	app.delete(
		"/:staticSiteId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::DELETE,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_static_site)),
		],
	);

	// List all linked URLs of a static site
	app.get(
		"/:staticSiteId/managed-urls",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission:
					permissions::workspace::infrastructure::static_site::INFO,
				resource: closure_as_pinned_box!(|mut context| {
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
			},
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
			current_live_upload: static_site.current_live_upload,
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
	let request_id = Uuid::new_v4();
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
		current_live_upload: static_site.current_live_upload,
	})
	.collect::<Vec<_>>();
	log::trace!("request_id: {} - Returning the list of all static sites for the workspace", request_id);

	context.success(ListStaticSitesResponse { static_sites });
	Ok(context)
}

/// # Description
/// This function is used to list of all the static sites upload history
/// required inputs:
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
///    success:
///    uploads: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_static_sites_upload_history(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();

	db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let uploads = db::get_static_site_upload_history(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.into_iter()
	.map(|deploy_history| StaticSiteUploadHistory {
		upload_id: deploy_history.id,
		message: deploy_history.message,
		uploaded_by: deploy_history.uploaded_by,
		created: DateTime(deploy_history.created),
	})
	.collect();

	context.success(ListStaticSiteUploadHistoryResponse { uploads });
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
async fn create_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - Creating a static site", request_id);
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let user_id = context.get_token_data().unwrap().user_id().clone();

	let CreateStaticSiteRequest {
		workspace_id: _,
		name,
		message,
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
		file,
		&message,
		&user_id,
		&config,
		&request_id,
	)
	.await?;

	log::trace!("request_id: {} - Static-site created", request_id);

	let _ = service::get_internal_metrics(
		context.get_database_connection(),
		"A static site has been created",
	)
	.await;
	context.success(CreateStaticSiteResponse { id });
	Ok(context)
}

/// # Description
/// This function is used to create a new static site
/// required inputs
/// auth token in the header
/// workspace_id,static_site_id and upload_id in parameter
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
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn revert_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();
	let upload_id =
		Uuid::parse_str(context.get_param(request_keys::UPLOAD_ID).unwrap())
			.unwrap();

	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();

	// check if upload_id is present in the deploy history
	db::get_static_site_upload_history_by_upload_id(
		context.get_database_connection(),
		&static_site_id,
		&upload_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!("request_id: {} - Reverting static site", request_id);

	let config = context.get_state().config.clone();

	db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	service::update_static_site_and_db_status(
		context.get_database_connection(),
		&workspace_id,
		&static_site_id,
		&upload_id,
		&StaticSiteDetails {},
		&config,
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	let managed_urls = db::get_managed_url_for_static_siite(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?;

	for managed_url in managed_urls {
		service::update_kubernetes_managed_url(
			&workspace_id,
			&ManagedUrl {
				id: managed_url.id,
				sub_domain: managed_url.sub_domain,
				domain_id: managed_url.domain_id,
				path: managed_url.path,
				url_type: ManagedUrlType::ProxyStaticSite {
					static_site_id: static_site_id.to_owned(),
				},
				is_configured: managed_url.is_configured,
			},
			&config,
			&request_id,
		)
		.await?;
	}

	context.success(RevertStaticSiteResponse {});
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
	let request_id = Uuid::new_v4();

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"request_id: {} - Starting a static site with id: {}",
		request_id,
		static_site_id
	);
	let config = context.get_state().config.clone();

	// Get current_live_upload from static_site
	let static_site = db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	// Check if upload_id is present or not
	if let Some(upload_id) = static_site.current_live_upload {
		service::update_static_site_and_db_status(
			context.get_database_connection(),
			&workspace_id,
			&static_site_id,
			&upload_id,
			&StaticSiteDetails {},
			&config,
			&request_id,
		)
		.await?;
	} else {
		return Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	};

	context.success(StartStaticSiteResponse {});
	Ok(context)
}

/// # Description
/// This function is used to get the status of domain set for static site
/// required inputs:
/// staticSiteId in the url
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
	let request_id = Uuid::new_v4();

	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();

	log::trace!(
		"Updating static site with id: {} and request_id: {}",
		static_site_id,
		request_id
	);
	let UpdateStaticSiteRequest { name, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.trim();

	// Check if resource(static site exists)
	db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	db::update_static_site_name(
		context.get_database_connection(),
		&static_site_id,
		name,
	)
	.await?;

	context.success(UpdateStaticSiteResponse {});
	Ok(context)
}

async fn upload_static_site(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();
	let request_id = Uuid::new_v4();
	let user_id = context.get_token_data().unwrap().user_id().clone();

	log::trace!(
		"Uploading the file for static site with id: {} and request_id: {}",
		static_site_id,
		request_id
	);
	let UploadStaticSiteRequest { file, message, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let config = context.get_state().config.clone();

	// Check if resource(static site exists)
	db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let upload_id = service::upload_static_site(
		context.get_database_connection(),
		&workspace_id,
		&static_site_id,
		&file,
		&message,
		&user_id,
		&config,
		&request_id,
	)
	.await?;

	context.commit_database_transaction().await?;

	log::trace!(
		"request_id: {} checking managed url for static_site with ID: {}",
		request_id,
		static_site_id
	);

	let managed_urls = db::get_managed_url_for_static_siite(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?;

	for managed_url in managed_urls {
		service::update_kubernetes_managed_url(
			&workspace_id,
			&ManagedUrl {
				id: managed_url.id,
				sub_domain: managed_url.sub_domain,
				domain_id: managed_url.domain_id,
				path: managed_url.path,
				url_type: ManagedUrlType::ProxyStaticSite {
					static_site_id: static_site_id.to_owned(),
				},
				is_configured: managed_url.is_configured,
			},
			&config,
			&request_id,
		)
		.await?;
	}

	context.success(UploadStaticSiteResponse { upload_id });
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
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();

	let request_id = Uuid::new_v4();
	log::trace!(
		"request_id: {} - Stopping a static site with id: {}",
		request_id,
		static_site_id
	);

	// stop the running site, if it exists
	let config = context.get_state().config.clone();
	service::stop_static_site(
		context.get_database_connection(),
		&workspace_id,
		&static_site_id,
		&config,
		&request_id,
	)
	.await?;

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
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let request_id = Uuid::new_v4();

	let static_site_id = Uuid::parse_str(
		context.get_param(request_keys::STATIC_SITE_ID).unwrap(),
	)
	.unwrap();

	let site = db::get_static_site_by_id(
		context.get_database_connection(),
		&static_site_id,
	)
	.await?
	.status(404)
	.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	log::trace!(
		"request_id: {} - Deleting the static site with id: {}",
		request_id,
		static_site_id
	);

	// stop and delete the container running the image, if it exists
	let config = context.get_state().config.clone();
	service::delete_static_site(
		context.get_database_connection(),
		&workspace_id,
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

	// Commiting transaction so that even if the mailing function fails the
	// resource should be deleted
	context.commit_database_transaction().await?;

	service::resource_delete_action_email(
		context.get_database_connection(),
		&site.name,
		&site.workspace_id,
		&ResourceType::StaticSite,
		&user_id,
	)
	.await?;

	context.success(DeleteStaticSiteResponse {});
	Ok(context)
}

async fn list_linked_urls(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();
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
			is_configured: url.is_configured,
		})
	})
	.collect();

	context.success(ListLinkedURLsResponse { urls });
	Ok(context)
}
