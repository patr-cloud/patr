use api_models::{
	models::workspace::{
		region::RegionStatus,
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
	utils::{DateTime, Uuid},
};
use axum::{
	extract::State,
	routing::{delete, get, post},
	Router,
};
use chrono::{Duration, Utc};
use sqlx::types::Json;

use crate::{
	app::App,
	db,
	error,
	models::rbac::{self},
	redis,
	service::{self, get_access_token_expiry},
	utils::{constants::request_keys, Error},
};

mod billing;
mod ci;
mod docker_registry;
mod domain;
mod infrastructure;
#[path = "rbac/mod.rs"]
mod rbac_routes;
mod region;
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
pub fn create_sub_route(app: &App) -> Router {
	let mut router = Router::new();

	router.route("/is-name-available", get(is_name_available));
	//  Route with plainTokenAUthenticator middleware
	router.route("/", post(create_new_workspace));
	//  Route with plainTokenAuthenticator middleware with a custom function for
	// check
	router.route("/:workspaceId/info", get(get_workspace_info));
	//  Route with resourceTokenAUthenticator middleware
	router.route("/:workspaceId", post(update_workspace_info));
	router.nest(
		"/:workspaceId/infrastructure",
		infrastructure::create_sub_route(app),
	);
	router.route(
		"/:workspaceId/docker-registry",
		docker_registry::create_sub_route(app),
	);
	router.nest("/:workspaceId/domain", domain::create_sub_route(app));
	router.nest("/:workspaceId/billing", billing::create_sub_route(app));
	router.nest("/:workspaceId/rbac", rbac_routes::create_sub_route(app));
	router.nest("/:workspaceId/secret", secret::create_sub_route(app));
	router.nest("/:workspaceId/ci", ci::create_sub_route(app));
	router.nest("/:workspaceId/region", region::create_sub_route(app));

	//  Route with resourceTokenAUthenticator middleware
	router.route("/:workspaceId", delete(delete_workspace));

	//  Route with resourceTokenAUthenticator middleware
	router.route("/:workspaceId/audit-log", get(get_workspace_audit_log));

	//  Route with resourceTokenAUthenticator middleware
	router.route(
		"/:workspaceId/audit-log/:resourceId",
		get(get_resource_audit_log),
	);

	router
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
	State(app): State<App>,
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

	if !access_token_data
		.workspace_permissions()
		.contains_key(&workspace_id) &&
		access_token_data.user_id() != god_user_id
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
		super_admin_id: workspace.super_admin_id,
		alert_emails: workspace.alert_emails,
		default_payment_method_id: workspace.default_payment_method_id,
		is_verified: !workspace.is_spam,
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
	State(app): State<App>,
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
	State(app): State<App>,
) -> Result<EveContext, Error> {
	let CreateNewWorkspaceRequest { workspace_name } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let workspace_name = workspace_name.trim().to_lowercase();

	let config = context.get_state().config.clone();

	let user_id = context.get_token_data().unwrap().user_id().clone();

	let alert_emails = if let Some(recovery_email) =
		db::get_recovery_email_for_user(
			context.get_database_connection(),
			&user_id,
		)
		.await?
	{
		vec![recovery_email]
	} else {
		vec![]
	};

	let workspace_id = service::create_workspace(
		context.get_database_connection(),
		&workspace_name,
		&user_id,
		false,
		&alert_emails,
		&config,
	)
	.await?;

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	redis::revoke_user_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&user_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

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
	State(app): State<App>,
) -> Result<EveContext, Error> {
	let UpdateWorkspaceInfoRequest {
		name,
		alert_emails,
		default_payment_method_id,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	if let Some(ref workspace_name) = name {
		let workspace_name = workspace_name.trim().to_lowercase();

		if workspace_name.is_empty() {
			// No parameters to update
			Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())?;
		}

		let allowed = service::is_workspace_name_allowed(
			context.get_database_connection(),
			&workspace_name,
			false,
		)
		.await?;

		if !allowed {
			Error::as_result()
				.status(400)
				.body(error!(INVALID_WORKSPACE_NAME).to_string())?;
		}
	}

	db::update_workspace_info(
		context.get_database_connection(),
		&workspace_id,
		name,
		alert_emails,
		default_payment_method_id,
	)
	.await?;

	context.success(UpdateWorkspaceInfoResponse {});
	Ok(context)
}

async fn delete_workspace(State(app): State<App>) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - requested to delete workspace", request_id);

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	// Make sure that a workspace with that ID exists. Users shouldn't be
	// allowed to delete a workspace that doesn't exist
	db::get_workspace_info(context.get_database_connection(), &workspace_id)
		.await?
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let namespace = workspace_id.as_str();

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

	let connected_git_providers =
		db::list_connected_git_providers_for_workspace(
			context.get_database_connection(),
			&workspace_id,
		)
		.await?;

	let ci_runners = db::get_runners_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	let regions = db::get_all_regions_for_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?;

	if !domains.is_empty() ||
		!docker_repositories.is_empty() ||
		!managed_database.is_empty() ||
		!deployments.is_empty() ||
		!static_site.is_empty() ||
		!managed_url.is_empty() ||
		!connected_git_providers.is_empty() ||
		!ci_runners.is_empty() ||
		!regions.is_empty()
	{
		return Err(Error::empty()
			.status(424)
			.body(error!(CANNOT_DELETE_WORKSPACE).to_string()));
	}

	for config in db::get_all_default_regions(context.get_database_connection())
		.await?
		.into_iter()
		.filter_map(|region| {
			if region.status == RegionStatus::Active {
				region.config_file.map(|Json(config)| config)
			} else {
				None
			}
		}) {
		service::delete_kubernetes_namespace(namespace, config, &request_id)
			.await?;
	}

	db::delete_workspace(
		context.get_database_connection(),
		&workspace_id,
		&Utc::now(),
	)
	.await?;

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	redis::revoke_workspace_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&workspace_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

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
	State(app): State<App>,
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
		date: DateTime(log.date),
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
	State(app): State<App>,
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
		date: DateTime(log.date),
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
