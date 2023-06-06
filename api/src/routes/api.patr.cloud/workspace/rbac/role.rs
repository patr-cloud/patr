use api_models::{
	models::workspace::rbac::role::{
		CreateNewRoleRequest,
		CreateNewRoleResponse,
		DeleteRoleResponse,
		GetRoleDetailsResponse,
		ListAllRolesResponse,
		ListUsersForRoleResponse,
		Role,
		UpdateRoleRequest,
		UpdateRoleResponse,
	},
	utils::Uuid,
};
use chrono::{Duration, Utc};
use eve_rs::{App as EveApp, AsError, Context, Error as _, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	pin_fn,
	redis::revoke_user_tokens_created_before_timestamp,
	service::get_access_token_expiry,
	utils::{constants::request_keys, Error, EveContext, EveMiddleware},
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
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	let mut sub_app = create_eve_app(app);

	// List all roles
	sub_app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::rbac::roles::LIST,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(list_all_roles)),
		],
	);

	// Create new role
	sub_app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::rbac::roles::CREATE,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(create_role)),
		],
	);

	// Get information of a role
	sub_app.get(
		"/:roleId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::rbac::roles::LIST,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(get_role_details)),
		],
	);

	// List all users with for a specific role in workspace
	sub_app.get(
		"/:roleId/users",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: true,
				permission: permissions::workspace::rbac::user::LIST,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if resource.is_none() {
						context
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(
				list_users_with_role_in_workspace
			)),
		],
	);

	// Update a role
	sub_app.put(
		"/:roleId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::rbac::roles::EDIT,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let role_id =
						context.get_param(request_keys::ROLE_ID).unwrap();
					let role_id = Uuid::parse_str(role_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let role = db::get_role_by_id(
						context.get_database_connection(),
						&role_id,
					)
					.await?
					.filter(|role| role.owner_id == workspace_id);

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if role.is_none() || resource.is_none() {
						context
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(update_role)),
		],
	);

	// Delete a role
	sub_app.delete(
		"/:roleId",
		[
			EveMiddleware::ResourceTokenAuthenticator {
				is_api_token_allowed: false,
				permission: permissions::workspace::rbac::roles::DELETE,
				resource: api_macros::closure_as_pinned_box!(|mut context| {
					let workspace_id =
						context.get_param(request_keys::WORKSPACE_ID).unwrap();
					let workspace_id = Uuid::parse_str(workspace_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;

					let role_id =
						context.get_param(request_keys::ROLE_ID).unwrap();
					let role_id = Uuid::parse_str(role_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let role = db::get_role_by_id(
						context.get_database_connection(),
						&role_id,
					)
					.await?
					.filter(|role| role.owner_id == workspace_id);

					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&workspace_id,
					)
					.await?;

					if role.is_none() || resource.is_none() {
						context
							.status(404)?
							.json(error!(RESOURCE_DOES_NOT_EXIST))
							.await?;
					}

					Ok((context, resource))
				}),
			},
			EveMiddleware::CustomFunction(pin_fn!(delete_role)),
		],
	);

	sub_app
}

/// # Description
/// This function is used to list all the roles available within the
/// workspace
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
///    roles:
///    [
///        {
///           roleId: ,
///           name: ,
///           description: -> only there if there some description present,
///        }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_all_roles(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id).unwrap();
	let roles = db::get_all_roles_in_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.map(|role| Role {
		id: role.id,
		name: role.name,
		description: role.description,
	})
	.collect::<Vec<_>>();

	context.success(ListAllRolesResponse { roles }).await?;
	Ok(context)
}

/// # Description
/// This function is used to describe the role
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
/// role id in the url
/// ```
/// {
///     roleId: ,
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
///    resourcePermissions:
///    [
///        {
///            id: ,
///            name: ,
///            descrpition: -> only available when there is some description given resource permission
///        }
///    ],
///    resourceTypePermissions:
///    [
///        {
///            id: ,
///            name: ,
///            descrpition: -> only available when there is some description given resource type permission
///        }
///    ]
///
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_role_details(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let role_id = context.get_param(request_keys::ROLE_ID).unwrap();
	let role_id = Uuid::parse_str(role_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// Check if the role exists
	let role = db::get_role_by_id(context.get_database_connection(), &role_id)
		.await?
		.filter(|role| role.owner_id == workspace_id)
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let permissions = db::get_permissions_for_role(
		context.get_database_connection(),
		&role_id,
	)
	.await?;

	context
		.success(GetRoleDetailsResponse {
			role: Role {
				id: role.id,
				name: role.name,
				description: role.description,
			},
			permissions,
		})
		.await?;
	Ok(context)
}

/// # Description
/// This function is used to create a new role
/// required inputs:
/// auth token in the header
/// workspace id in the url
/// ```
/// {
///     name: ,
///     description: , -> not mandatory
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
///    roleId:
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn create_role(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;

	let CreateNewRoleRequest {
		name,
		description,
		permissions,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let name = name.trim();

	let role_id =
		db::generate_new_role_id(context.get_database_connection()).await?;

	db::create_role(
		context.get_database_connection(),
		&role_id,
		name,
		&description,
		&workspace_id,
	)
	.await?;
	db::insert_permissions_for_role(
		context.get_database_connection(),
		&role_id,
		&permissions,
	)
	.await?;

	context
		.success(CreateNewRoleResponse { id: role_id })
		.await?;
	Ok(context)
}

async fn list_users_with_role_in_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.unwrap();
	let role_id =
		Uuid::parse_str(context.get_param(request_keys::ROLE_ID).unwrap())
			.unwrap();

	db::get_role_by_id(context.get_database_connection(), &role_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let users = db::list_all_users_for_role_in_workspace(
		context.get_database_connection(),
		&workspace_id,
		&role_id,
	)
	.await?;

	context.success(ListUsersForRoleResponse { users }).await?;
	Ok(context)
}

/// # Description
/// This function is used to update the permissions of the role
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
/// role id in the url
/// ```
/// {
///     resourcePermissions: [],
///     resourceTypePermissions: []
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
async fn update_role(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let role_id = context.get_param(request_keys::ROLE_ID).unwrap();
	let role_id = Uuid::parse_str(role_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let UpdateRoleRequest {
		name,
		description,
		permissions,
		..
	} = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let name = name.as_deref().map(|name| name.trim());

	db::update_role_name_and_description(
		context.get_database_connection(),
		&role_id,
		name,
		description.as_deref(),
	)
	.await?;

	let associated_users = db::get_all_users_with_role(
		context.get_database_connection(),
		&role_id,
	)
	.await?;

	if let Some(permissions) = permissions {
		db::remove_all_permissions_for_role(
			context.get_database_connection(),
			&role_id,
		)
		.await?;
		db::insert_permissions_for_role(
			context.get_database_connection(),
			&role_id,
			&permissions,
		)
		.await?;
	}

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	for user in associated_users {
		revoke_user_tokens_created_before_timestamp(
			context.get_redis_connection(),
			&user.id,
			&Utc::now(),
			Some(&ttl),
		)
		.await?;
	}

	context.success(UpdateRoleResponse {}).await?;
	Ok(context)
}

/// # Description
/// This function is used to delete a role
/// required inputs:
/// auth token in the authorization headers
/// workspace id in the url
/// role id in the url
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
async fn delete_role(
	mut context: EveContext,
	_: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	let role_id = context.get_param(request_keys::ROLE_ID).unwrap();
	let role_id = Uuid::parse_str(role_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let associated_users = db::get_all_users_with_role(
		context.get_database_connection(),
		&role_id,
	)
	.await?;

	if !associated_users.is_empty() {
		return Err(Error::empty()
			.status(400)
			.body(error!(RESOURCE_IN_USE).to_string()));
	}

	// Delete role
	db::delete_role(context.get_database_connection(), &role_id).await?;

	context.success(DeleteRoleResponse {}).await?;
	Ok(context)
}
