use std::collections::HashMap;

use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use hex::ToHex;
use serde_json::{json, Map, Value};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::rbac::permissions,
	pin_fn,
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
	let mut sub_app = create_eve_app(app);

	// List all roles
	sub_app.get(
		"/roles",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::VIEW_ROLES,
				api_macros::closure_as_pinned_box!(|mut context| {
					let organisation_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
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
	sub_app.get(
		"/permissions",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::VIEW_ROLES,
				api_macros::closure_as_pinned_box!(|mut context| {
					let organisation_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
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
			EveMiddleware::CustomFunction(pin_fn!(list_all_permissions)),
		],
	);
	sub_app.get(
		"/resourceTypes",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::VIEW_ROLES,
				api_macros::closure_as_pinned_box!(|mut context| {
					let organisation_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
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
			EveMiddleware::CustomFunction(pin_fn!(list_all_resource_types)),
		],
	);

	// Create new role
	sub_app.post(
		"/role",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::CREATE_ROLE,
				api_macros::closure_as_pinned_box!(|mut context| {
					let organisation_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
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
	// List permissions for a role
	sub_app.get(
		"/role/:roleId/permissions",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::VIEW_ROLES,
				api_macros::closure_as_pinned_box!(|mut context| {
					let organisation_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_permissions_for_role)),
		],
	);
	// Update permissions for a role
	sub_app.post(
		"/role/:roleId/permissions",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::EDIT_ROLE,
				api_macros::closure_as_pinned_box!(|mut context| {
					let organisation_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
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
			EveMiddleware::CustomFunction(pin_fn!(update_role_permissions)),
		],
	);
	sub_app.delete(
		"/role/:roleId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::DELETE_ROLE,
				api_macros::closure_as_pinned_box!(|mut context| {
					let organisation_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
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
			EveMiddleware::CustomFunction(pin_fn!(delete_role)),
		],
	);

	// get resource info
	sub_app.get(
		"/resource/:resourceId/info",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::VIEW_ROLES,
				api_macros::closure_as_pinned_box!(|mut context| {
					let organisation_id = context
						.get_param(request_keys::ORGANISATION_ID)
						.unwrap();
					let organisation_id = hex::decode(&organisation_id)
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?;
					let resource = db::get_resource_by_id(
						context.get_database_connection(),
						&organisation_id,
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
			EveMiddleware::CustomFunction(pin_fn!(get_resource_info)),
		],
	);

	sub_app
}

/// # Description
/// This function is used to list all the roles available within the
/// organisation
/// required inputs:
/// auth token in the authorization headers
/// orgnisation id in the url
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
	let organisation_id =
		context.get_param(request_keys::ORGANISATION_ID).unwrap();
	let organisation_id = hex::decode(organisation_id).unwrap();
	let roles = db::get_all_organisation_roles(
		context.get_database_connection(),
		&organisation_id,
	)
	.await?
	.into_iter()
	.map(|role| {
		let role_id = role.id.encode_hex::<String>();
		if let Some(description) = role.description {
			json!({
				request_keys::ROLE_ID: role_id,
				request_keys::NAME: role.name,
				request_keys::DESCRIPTION: description,
			})
		} else {
			json!({
				request_keys::ROLE_ID: role_id,
				request_keys::NAME: role.name,
			})
		}
	})
	.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ROLES: roles
	}));
	Ok(context)
}

/// # Description
/// This function is used to list all the permissions available in the API
/// required inputs:
/// auth token in the authorization headers
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
///    permissions:
///    [
///       {
///           id: ,
///           name: ,
///           description: -> only available when there is some description given about the permission,
///       }
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_all_permissions(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let permissions =
		db::get_all_permissions(context.get_database_connection())
			.await?
			.into_iter()
			.map(|permission| {
				let permission_id = permission.id.encode_hex::<String>();
				if let Some(description) = permission.description {
					json!({
						request_keys::PERMISSION_ID: permission_id,
						request_keys::NAME: permission.name,
						request_keys::DESCRIPTION: description,
					})
				} else {
					json!({
						request_keys::PERMISSION_ID: permission_id,
						request_keys::NAME: permission.name,
					})
				}
			})
			.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::PERMISSIONS: permissions
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the list of all the resource types available in
/// the API
/// required inputs:
/// auth token in the description
/// organisation id in the url
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
///    resourceTypes:
///    [
///       resourceTypeId: ,
///       name: ,
///       description: -> only available when there is a description about the resource type
///    ]
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn list_all_resource_types(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let resource_types =
		db::get_all_resource_types(context.get_database_connection())
			.await?
			.into_iter()
			.map(|resource_type| {
				let resource_type_id = resource_type.id.encode_hex::<String>();
				if let Some(description) = resource_type.description {
					json!({
						request_keys::RESOURCE_TYPE_ID: resource_type_id,
						request_keys::NAME: resource_type.name,
						request_keys::DESCRIPTION: description,
					})
				} else {
					json!({
						request_keys::RESOURCE_TYPE_ID: resource_type_id,
						request_keys::NAME: resource_type.name,
					})
				}
			})
			.collect::<Vec<_>>();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::RESOURCE_TYPES: resource_types
	}));
	Ok(context)
}

/// # Description
/// This function is used to get the list of permissions for the role
/// required inputs:
/// auth token in the authorization headers
/// organisation id in the url
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
async fn get_permissions_for_role(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let role_id = context.get_param(request_keys::ROLE_ID).unwrap();
	let role_id = hex::decode(role_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// Check if the role exists
	let _ = db::get_role_by_id(context.get_database_connection(), &role_id)
		.await?
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let resource_permissions = db::get_permissions_on_resources_for_role(
		context.get_database_connection(),
		&role_id,
	)
	.await?;
	let resource_type_permissions =
		db::get_permissions_on_resource_types_for_role(
			context.get_database_connection(),
			&role_id,
		)
		.await?;

	let mut resource_map = Map::new();
	let mut resource_type_map = Map::new();

	for (resource_id, permissions) in resource_permissions {
		resource_map.insert(
			resource_id.encode_hex::<String>(),
			Value::Array(
				permissions
					.into_iter()
					.map(|permission| {
						if let Some(description) = permission.description {
							json!({
								request_keys::ID: permission.id,
								request_keys::NAME: permission.name,
								request_keys::DESCRIPTION: description,
							})
						} else {
							json!({
								request_keys::ID: permission.id,
								request_keys::NAME: permission.name,
							})
						}
					})
					.collect::<Vec<_>>(),
			),
		);
	}
	for (resource_id, permissions) in resource_type_permissions {
		resource_type_map.insert(
			resource_id.encode_hex::<String>(),
			Value::Array(
				permissions
					.into_iter()
					.map(|permission| {
						if let Some(description) = permission.description {
							json!({
								request_keys::ID: permission.id,
								request_keys::NAME: permission.name,
								request_keys::DESCRIPTION: description,
							})
						} else {
							json!({
								request_keys::ID: permission.id,
								request_keys::NAME: permission.name,
							})
						}
					})
					.collect::<Vec<_>>(),
			),
		);
	}

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::RESOURCE_PERMISSIONS: Value::Object(resource_map),
		request_keys::RESOURCE_TYPE_PERMISSIONS: Value::Object(resource_type_map),
	}));
	Ok(context)
}

/// # Description
/// This function is used to create a new role
/// required inputs:
/// auth token in the header
/// organisation id in the url
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
	let organisation_id = hex::decode(
		context
			.get_param(request_keys::ORGANISATION_ID)
			.unwrap()
			.clone(),
	)
	.status(400)
	.body(error!(WRONG_PARAMETERS).to_string())?;

	let body = context.get_body_object().clone();
	let name = body
		.get(request_keys::NAME)
		.map(|value| value.as_str())
		.flatten()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;
	let description = body
		.get(request_keys::DESCRIPTION)
		.map(|value| {
			value
				.as_str()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string())
		})
		.transpose()?;
	let role_id = db::generate_new_role_id(context.get_database_connection())
		.await?
		.as_bytes()
		.to_vec();
	db::create_role(
		context.get_database_connection(),
		&role_id,
		name,
		description,
		&organisation_id,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::ROLE_ID: role_id.encode_hex::<String>(),
	}));
	Ok(context)
}

/// # Description
/// This function is used to update the permissions of the role
/// required inputs:
/// auth token in the authorization headers
/// organisation id in the url
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
async fn update_role_permissions(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let role_id = context.get_param(request_keys::ROLE_ID).unwrap();
	let role_id = hex::decode(role_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let body = context.get_body_object().clone();

	let resource_permissions_map = if let Some(Value::Object(permissions)) =
		body.get(request_keys::RESOURCE_PERMISSIONS)
	{
		permissions
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};
	let resource_type_permissions_map =
		if let Some(Value::Object(permissions)) =
			body.get(request_keys::RESOURCE_TYPE_PERMISSIONS)
		{
			permissions
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};

	let mut resource_permissions = HashMap::new();
	let mut resource_type_permissions = HashMap::new();

	for (resource_id, permissions) in resource_permissions_map {
		let resource_id = if let Ok(resource_id) = hex::decode(resource_id) {
			resource_id
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};
		let permissions = if let Value::Array(permissions) = permissions {
			permissions
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};
		let mut permissions_values = Vec::with_capacity(permissions.len());
		for permission_id in permissions {
			let permission_id = if let Value::String(permission) = permission_id
			{
				permission
			} else {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			};
			if let Ok(permission_id) = hex::decode(permission_id) {
				permissions_values.push(permission_id);
			} else {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			}
		}
		resource_permissions.insert(resource_id, permissions_values);
	}
	for (resource_type_id, permissions) in resource_type_permissions_map {
		let resource_type_id =
			if let Ok(resource_type_id) = hex::decode(resource_type_id) {
				resource_type_id
			} else {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			};
		let permissions = if let Value::Array(permissions) = permissions {
			permissions
		} else {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		};
		let mut permissions_values = Vec::with_capacity(permissions.len());
		for permission_id in permissions {
			let permission_id = if let Value::String(permission) = permission_id
			{
				permission
			} else {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			};
			if let Ok(permission_id) = hex::decode(permission_id) {
				permissions_values.push(permission_id);
			} else {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			}
		}
		resource_type_permissions.insert(resource_type_id, permissions_values);
	}

	db::remove_all_permissions_for_role(
		context.get_database_connection(),
		&role_id,
	)
	.await?;
	db::insert_resource_permissions_for_role(
		context.get_database_connection(),
		&role_id,
		&resource_permissions,
	)
	.await?;
	db::insert_resource_type_permissions_for_role(
		context.get_database_connection(),
		&role_id,
		&resource_type_permissions,
	)
	.await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to delete a role
/// required inputs:
/// auth token in the authorization headers
/// organisation id in the url
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
	let role_id = hex::decode(role_id)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	// Remove all users who belong to this role
	db::remove_all_users_from_role(context.get_database_connection(), &role_id)
		.await?;
	// Delete role
	db::delete_role(context.get_database_connection(), &role_id).await?;

	context.json(json!({
		request_keys::SUCCESS: true
	}));
	Ok(context)
}

/// # Description
/// This function is used to get details about the resource
/// required inputs:
/// auth token in the authorization headers
/// organisation id in the url
/// ```
/// {
///     resourceId:
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
///    resource:
///    {
///        id: ,
///        name: ,
///        resourceType: ,
///    }
/// }
/// ```
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
async fn get_resource_info(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let resource_id_string = context
		.get_param(request_keys::RESOURCE_ID)
		.unwrap()
		.clone();
	let resource_id = hex::decode(&resource_id_string)
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	let resource =
		db::get_resource_by_id(context.get_database_connection(), &resource_id)
			.await?
			.status(400)
			.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;
	let resource_type = db::get_resource_type_for_resource(
		context.get_database_connection(),
		&resource.id,
	)
	.await?
	.unwrap();

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::RESOURCE: {
			request_keys::ID: resource_id_string,
			request_keys::NAME: resource.name,
			request_keys::RESOURCE_TYPE: resource_type.name,
		}
	}));
	Ok(context)
}
