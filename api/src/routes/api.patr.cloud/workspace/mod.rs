use api_models::{
	models::workspace::{
		CreateNewWorkspaceRequest,
		CreateNewWorkspaceResponse,
		GetAddUserToWorkspaceRequest,
		GetWorkspaceInfoResponse,
		IsWorkspaceNameAvailableRequest,
		IsWorkspaceNameAvailableResponse,
		UpdateWorkspaceInfoRequest,
		UpdateWorkspaceInfoResponse,
		Workspace,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::{self, permissions},
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

mod docker_registry;
mod domain;
mod infrastructure;
#[path = "./rbac.rs"]
mod rbac_routes;

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
		"/:workspaceId/info",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(get_workspace_info)),
		],
	);
	sub_app.post(
		"/:workspaceId/info",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::EDIT_INFO,
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
	sub_app.use_sub_app("/:workspaceId/rbac", rbac_routes::create_sub_app(app));

	sub_app.get(
		"/is-name-available",
		[EveMiddleware::CustomFunction(pin_fn!(is_name_available))],
	);
	sub_app.post(
		"/",
		[
			EveMiddleware::PlainTokenAuthenticator,
			EveMiddleware::CustomFunction(pin_fn!(create_new_workspace)),
		],
	);

	sub_app.post(
		"/:workspaceId/add-user",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::user::CREATE,
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
			EveMiddleware::CustomFunction(pin_fn!(add_user_to_workspace)),
		],
	);

	sub_app.delete(
		"/:workspaceId/delete-user/:userId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::user::DELETE,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_user_from_workspace)),
		],
	);

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
			EveMiddleware::CustomFunction(pin_fn!(delete_workspace)),
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

	context.success(UpdateWorkspaceInfoResponse {});
	Ok(context)
}

async fn add_user_to_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let GetAddUserToWorkspaceRequest { user_id, role_id } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	// check if user with above user_id exits
	let user =
		db::get_user_by_user_id(context.get_database_connection(), &user_id)
			.await?;

	let user_exists = user.is_some();

	// check if role with above role_id exits
	let role =
		db::get_role_by_id(context.get_database_connection(), &role_id).await?;

	let role_exists = role.is_some();

	if user_exists && role_exists {
		db::add_user_to_workspace_with_role(
			context.get_database_connection(),
			&user_id,
			&workspace_id,
			&role_id,
		)
		.await?;
	}

	context.success({});
	Ok(context)
}

async fn delete_user_from_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let user_id = context.get_param(request_keys::USER_ID).unwrap();

	let workspace_id = Uuid::parse_str(workspace_id).unwrap();
	let user_id = Uuid::parse_str(user_id).unwrap();

	db::delete_user_from_workspace(
		context.get_database_connection(),
		&user_id,
		&workspace_id,
	)
	.await?;

	context.success({});
	Ok(context)
}

async fn delete_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	log::trace!("request_id: {} - requested to delete workspace", request_id);

	let UpdateWorkspaceInfoRequest { name, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let name = name.trim().to_lowercase();

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();

	let name = format!("patr-deleted-{}-{}", workspace_id, name);
	let namespace_name = workspace_id.as_str();

	let config = context.get_state().config.clone();

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

	if deployments.is_empty() &&
		static_site.is_empty() &&
		managed_url.is_empty() &&
		domains.is_empty() &&
		managed_database.is_empty()
	{
		service::delete_kubernetes_namespace(
			namespace_name,
			&config,
			&request_id,
		)
		.await?;

		db::update_workspace_name(
			context.get_database_connection(),
			&workspace_id,
			&name,
		)
		.await?;

		log::trace!("request_id: {} - deleted the workspace", request_id);
		context.success(UpdateWorkspaceInfoResponse {});
		Ok(context)
	} else {
		return Error::as_result()
			.status(500)
			.body(error!(CANNOT_DELETE_WORKSPACE).to_string());
	}
}
