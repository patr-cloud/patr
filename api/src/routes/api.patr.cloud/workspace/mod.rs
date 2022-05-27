use api_models::{
	models::workspace::{
		CreateNewWorkspaceRequest,
		CreateNewWorkspaceResponse,
		DeleteWorkspaceResponse,
		GetWorkspaceAuditLogResponse,
		GetWorkspaceInfoResponse,
		IsWorkspaceNameAvailableRequest,
		IsWorkspaceNameAvailableResponse,
		UpdateWorkspaceInfoRequest,
		UpdateWorkspaceInfoResponse,
		Workspace,
		WorkspaceAuditLog,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::{
		rbac::{self, permissions, PERMISSIONS},
		WorkspaceMetadata,
	},
	pin_fn,
	redis,
	service::{self, get_access_token_expiry},
	utils::{
		audit_logger::AuditLogData,
		constants::request_keys,
		get_current_time_millis,
		Error,
		ErrorData,
		EveContext,
		EveMiddleware,
	},
};

mod billing;
mod ci;
mod docker_registry;
mod domain;
mod infrastructure;
#[path = "rbac/mod.rs"]
mod rbac_routes;
mod secret;

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions. This file
/// contains major enpoints which are meant for the workspaces, and all other
/// endpoints will come uder these
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
	let mut sub_app = create_eve_app(app);

	sub_app.get(
		"/is-name-available",
		[EveMiddleware::CustomFunction(pin_fn!(is_name_available))],
	);
	sub_app.post(
		"/",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::AuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(create_new_workspace)),
		],
	);
	sub_app.get(
		"/:workspaceId/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_workspace_info)),
		],
	);
	sub_app.post(
		"/:workspaceId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::AuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(update_workspace_info)),
		],
	);
	sub_app.use_sub_app(
		"/:workspaceId/infrastructure",
		infrastructure::create_sub_app(app),
	);
	sub_app.use_sub_app(
		"/:workspaceId/docker-registry",
		docker_registry::create_sub_app(app),
	);
	sub_app.use_sub_app("/:workspaceId/domain", domain::create_sub_app(app));
	sub_app.use_sub_app("/:workspaceId/billing", billing::create_sub_app(app));
	sub_app.use_sub_app("/:workspaceId/rbac", rbac_routes::create_sub_app(app));
	sub_app.use_sub_app("/:workspaceId/secret", secret::create_sub_app(app));
	sub_app.use_sub_app("/:workspaceId/ci", ci::create_sub_app(app));

	sub_app.delete(
		"/:workspaceId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::DELETE,
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
			EveMiddleware::AuditLogger,
			EveMiddleware::CustomFunction(pin_fn!(delete_workspace)),
		],
	);

	sub_app.get(
		"/:workspaceId/audit-log",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
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
			EveMiddleware::CustomFunction(pin_fn!(get_workspace_audit_log)),
		],
	);

	sub_app.get(
		"/:workspaceId/audit-log/:resourceId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT,
				api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id_string =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id_string)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource_id = context
						.get_param(request_keys::MANAGED_URL_ID)
						.unwrap();
					let resource_id = Uuid::parse_str(resource_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&resource_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_resource_audit_log)),
		],
	);

	sub_app
}

/// # Description
/// This function is used to get details about an workspace
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
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
///    workspaceId: ,
///    name: ,
///    active: true or false,
///    created:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_workspace_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id_string = context
		.get_param(request_keys::WORKSPACE_ID)
		.unwrap()
		.clone();
	let workspace_id = Uuid::parse_str(&workspace_id_string)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let access_token_data = context.get_token_data().unwrap();
	let god_user_id = rbac::GOD_USER_ID.get().unwrap();

	if !access_token_data.workspaces.contains_key(&workspace_id) &&
		&access_token_data.user.id != god_user_id
	{
		Error::as_result()
			.status(404)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	}

	let workspace = db::get_workspace_info(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.map(|workspace| Workspace {
		id: workspace.id,
		name: workspace.name,
		active: workspace.active,
	})
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	context.success(GetWorkspaceInfoResponse { workspace });
	Ok(context)
}

/// # Description
/// This function is used to check if the workspace name is available or not
/// required inputs:
/// auth token in the authorization headers
/// ```
/// {
///     name:
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
///    allowed: true or false
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn is_name_available(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let IsWorkspaceNameAvailableRequest { name } = context
		.get_query_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let workspace_name = name.trim().to_lowercase();

	let available = service::is_workspace_name_allowed(
		context.get_database_connection(),
		&workspace_name,
		false,
	)
	.await?;

	context.success(IsWorkspaceNameAvailableResponse { available });
	Ok(context)
}

/// # Description
/// This function is used to create new workspace
/// required inputs:
/// auth token in the authorization headers
/// ```
/// {
///     workspaceName:
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
///    workspaceId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create_new_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let CreateNewWorkspaceRequest { workspace_name } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let workspace_name = workspace_name.trim().to_lowercase();

	let config = context.get_state().config.clone();

	let user_id = context.get_token_data().unwrap().user.id.clone();
	let workspace_id = service::create_workspace(
		context.get_database_connection(),
		&workspace_name,
		&user_id,
		false,
		&config,
	)
	.await?;

	let ttl = (get_access_token_expiry() / 1000) as usize + (2 * 60 * 60); // 2 hrs buffer time
	redis::revoke_user_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&user_id,
		get_current_time_millis(),
		Some(ttl),
	)
	.await?;

	context.set_audit_log_data(AuditLogData::WorkspaceResource {
		workspace_id: workspace_id.clone(),
		resource_id: workspace_id.clone(),
		action_id: PERMISSIONS
			.get()
			.and_then(|map| map.get(permissions::workspace::EDIT).cloned())
			.unwrap(),
		metadata: Some(serde_json::to_value(WorkspaceMetadata::Create {
			name: workspace_name,
		})?),
	});

	context.success(CreateNewWorkspaceResponse { workspace_id });
	Ok(context)
}

/// # Description
/// This function is used to update the workspace details
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
/// ```
/// {
///     name:
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
async fn update_workspace_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let UpdateWorkspaceInfoRequest { name, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.trim().to_lowercase();

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	if name.is_empty() {
		// No parameters to update
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}

	let allowed = service::is_workspace_name_allowed(
		context.get_database_connection(),
		&name,
		false,
	)
	.await?;
	if !allowed {
		Error::as_result()
			.status(400)
			.body(error!(INVALID_WORKSPACE_NAME).to_string())?;
	}

	db::update_workspace_name(
		context.get_database_connection(),
		&workspace_id,
		&name,
	)
	.await?;

	context.set_audit_log_data(AuditLogData::WorkspaceResource {
		workspace_id: workspace_id.clone(),
		resource_id: workspace_id,
		action_id: PERMISSIONS
			.get()
			.and_then(|map| map.get(permissions::workspace::EDIT).cloned())
			.unwrap(),
		metadata: Some(serde_json::to_value(WorkspaceMetadata::Update {
			name,
		})?),
	});

	context.success(UpdateWorkspaceInfoResponse {});
	Ok(context)
}

async fn delete_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = context.get_request_id().clone();

	log::trace!("request_id: {} - requested to delete workspace", request_id);

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let workspace = db::get_workspace_info(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let name = format!("patr-deleted: {}@{}", workspace_id, workspace.name);
	let namespace = workspace_id.as_str();

	let config = context.get_state().config.clone();

	let domains = db::get_domains_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	let managed_database = db::get_all_database_clusters_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	let deployments = db::get_deployments_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	let static_site = db::get_static_sites_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	let managed_url = db::get_all_managed_urls_in_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	let docker_repositories = db::get_docker_repositories_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	if !domains.is_empty() ||
		!docker_repositories.is_empty() ||
		!managed_database.is_empty() ||
		!deployments.is_empty() ||
		!static_site.is_empty() ||
		!managed_url.is_empty()
	{
		return Err(Error::empty()
			.status(424)
			.body(error!(CANNOT_DELETE_WORKSPACE).to_string()));
	}

	service::delete_kubernetes_namespace(namespace, &config, &request_id)
		.await?;

	db::update_workspace_name(
		context.get_database_connection(),
		&workspace_id,
		&name,
	)
	.await?;

	let ttl = (get_access_token_expiry() / 1000) as usize + (2 * 60 * 60); // 2 hrs buffer time
	redis::revoke_workspace_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&workspace_id,
		get_current_time_millis(),
		Some(ttl),
	)
	.await?;

	context.set_audit_log_data(AuditLogData::WorkspaceResource {
		workspace_id: workspace_id.clone(),
		resource_id: workspace_id,
		action_id: PERMISSIONS
			.get()
			.and_then(|map| map.get(permissions::workspace::DELETE).cloned())
			.unwrap(),
		metadata: Some(serde_json::to_value(WorkspaceMetadata::Delete {})?),
	});

	log::trace!("request_id: {} - deleted the workspace", request_id);
	context.success(DeleteWorkspaceResponse {});
	Ok(context)
}

/// # Description
/// This function is used to retrieve the list of workspace audit logs
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
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
///    workspaceAuditLogs: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_workspace_audit_log(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let workspace_audit_logs = db::get_workspace_audit_logs(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|log| WorkspaceAuditLog {
		id: log.id,
		date: log.date,
		ip_address: log.ip_address,
		workspace_id: log.workspace_id,
		user_id: log.user_id,
		login_id: log.login_id,
		resource_id: log.resource_id,
		action: log.action,
		request_id: log.request_id,
		metadata: log.metadata,
		patr_action: log.patr_action,
		request_success: log.success,
	})
	.collect();

	context.success(GetWorkspaceAuditLogResponse {
		audit_logs: workspace_audit_logs,
	});
	Ok(context)
}

/// # Description
/// This function is used to retrieve the list resource audit logs
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
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
///    workspaceAuditLogs: []
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_resource_audit_log(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let resource_id = context.get_param(request_keys::RESOURCE_ID).unwrap();
	let resource_id = Uuid::parse_str(resource_id).unwrap();

	let workspace_audit_logs = db::get_resource_audit_logs(
		context.get_database_connection(),
		&resource_id,
	)
	.await?
	.into_iter()
	.map(|log| WorkspaceAuditLog {
		id: log.id,
		date: log.date,
		ip_address: log.ip_address,
		workspace_id: log.workspace_id,
		user_id: log.user_id,
		login_id: log.login_id,
		resource_id: log.resource_id,
		action: log.action,
		request_id: log.request_id,
		metadata: log.metadata,
		patr_action: log.patr_action,
		request_success: log.success,
	})
	.collect();

	context.success(GetWorkspaceAuditLogResponse {
		audit_logs: workspace_audit_logs,
	});
	Ok(context)
}
