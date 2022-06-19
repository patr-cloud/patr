use api_models::{
	models::workspace::rbac::role::{
		CreateNewRoleRequest,
		CreateNewRoleResponse,
		DeleteRoleResponse,
		GetRoleDetailsResponse,
		ListAllRolesResponse,
		Role,
		UpdateRoleRequest,
		UpdateRoleResponse,
	},
	utils::Uuid,
};
use eve_rs::{App as EveApp, AsError, Context, NextHandler};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	pin_fn,
	redis::revoke_user_tokens_created_before_timestamp,
	service::get_access_token_expiry,
	utils::{
		constants::request_keys,
		get_current_time_millis,
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
	let mut sub_app = create_eve_app(app);

	// List all roles
	sub_app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::roles::LIST,
				api_macros::closure_as_pinned_box!(|mut context| {
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
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(list_all_roles)),
		],
	);

	// Create new role
	sub_app.post(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::roles::CREATE,
				api_macros::closure_as_pinned_box!(|mut context| {
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
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(create_role)),
		],
	);

	// Get information of a role
	sub_app.get(
		"/:roleId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::roles::LIST,
				api_macros::closure_as_pinned_box!(|mut context| {
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
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(get_role_details)),
		],
	);

	// Update a role
	sub_app.put(
		"/:roleId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::roles::EDIT,
				api_macros::closure_as_pinned_box!(|mut context| {
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
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(update_role)),
		],
	);

	// Delete a role
	sub_app.delete(
		"/:roleId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::roles::DELETE,
				api_macros::closure_as_pinned_box!(|mut context| {
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
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
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
	_: NextHandler<EveContext, ErrorData>,
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

	context.success(ListAllRolesResponse { roles });
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
	_: NextHandler<EveContext, ErrorData>,
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

	let block_resource_permissions =
		db::get_blocked_permissions_on_resources_for_role(
			context.get_database_connection(),
			&role_id,
		)
		.await?
		.into_iter()
		.map(|(key, value)| {
			(
				key,
				value.into_iter().map(|permission| permission.id).collect(),
			)
		})
		.collect();

	let allow_resource_permissions =
		db::get_allowed_permissions_on_resources_for_role(
			context.get_database_connection(),
			&role_id,
		)
		.await?
		.into_iter()
		.map(|(key, value)| {
			(
				key,
				value.into_iter().map(|permission| permission.id).collect(),
			)
		})
		.collect();

	let allow_resource_type_permissions =
		db::get_allowed_permissions_on_resource_types_for_role(
			context.get_database_connection(),
			&role_id,
		)
		.await?
		.into_iter()
		.map(|(key, value)| {
			(
				key,
				value.into_iter().map(|permission| permission.id).collect(),
			)
		})
		.collect();

	context.success(GetRoleDetailsResponse {
		role: Role {
			id: role.id,
			name: role.name,
			description: role.description,
		},
		block_resource_permissions,
		allow_resource_permissions,
		allow_resource_type_permissions,
	});
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
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id =
		Uuid::parse_str(context.get_param(request_keys::WORKSPACE_ID).unwrap())
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;

	let CreateNewRoleRequest {
		name,
		description,
		block_resource_permissions,
		allow_resource_permissions,
		allow_resource_type_permissions,
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
	db::insert_blocked_resource_permissions_for_role(
		context.get_database_connection(),
		&role_id,
		&block_resource_permissions,
	)
	.await?;
	db::insert_allowed_resource_permissions_for_role(
		context.get_database_connection(),
		&role_id,
		&allow_resource_permissions,
	)
	.await?;
	db::insert_allowed_resource_type_permissions_for_role(
		context.get_database_connection(),
		&role_id,
		&allow_resource_type_permissions,
	)
	.await?;

	context.success(CreateNewRoleResponse { id: role_id });
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
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let role_id = context.get_param(request_keys::ROLE_ID).unwrap();
	let role_id = Uuid::parse_str(role_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let UpdateRoleRequest {
		name,
		description,
		block_resource_permissions,
		allow_resource_permissions,
		allow_resource_type_permissions,
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

	if let (
		Some(block_resource_permissions),
		Some(allow_resource_permissions),
		Some(allow_resource_type_permissions),
	) = (
		block_resource_permissions,
		allow_resource_permissions,
		allow_resource_type_permissions,
	) {
		db::remove_all_permissions_for_role(
			context.get_database_connection(),
			&role_id,
		)
		.await?;
		db::insert_blocked_resource_permissions_for_role(
			context.get_database_connection(),
			&role_id,
			&block_resource_permissions,
		)
		.await?;
		db::insert_allowed_resource_permissions_for_role(
			context.get_database_connection(),
			&role_id,
			&allow_resource_permissions,
		)
		.await?;
		db::insert_allowed_resource_type_permissions_for_role(
			context.get_database_connection(),
			&role_id,
			&allow_resource_type_permissions,
		)
		.await?;
	}

	let ttl = (get_access_token_expiry() / 1000) as usize + (2 * 60 * 60); // 2 hrs buffer time
	for user in associated_users {
		revoke_user_tokens_created_before_timestamp(
			context.get_redis_connection(),
			&user.id,
			get_current_time_millis(),
			Some(ttl),
		)
		.await?;
	}

	context.success(UpdateRoleResponse {});
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
	_: NextHandler<EveContext, ErrorData>,
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

	// Remove all users who belong to this role
	db::remove_all_users_from_role(context.get_database_connection(), &role_id)
		.await?;
	// Delete role
	db::delete_role(context.get_database_connection(), &role_id).await?;

	let ttl = (get_access_token_expiry() / 1000) as usize + (2 * 60 * 60); // 2 hrs buffer time
	for user in associated_users {
		revoke_user_tokens_created_before_timestamp(
			context.get_redis_connection(),
			&user.id,
			get_current_time_millis(),
			Some(ttl),
		)
		.await?;
	}

	context.success(DeleteRoleResponse {});
	Ok(context)
}
