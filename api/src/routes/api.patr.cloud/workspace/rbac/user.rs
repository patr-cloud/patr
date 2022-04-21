use api_models::{
	models::workspace::rbac::user::{
		AddUserToWorkspaceRequest,
		AddUserToWorkspaceResponse,
		ListUsersWithRolesInWorkspaceResponse,
		RemoveUserFromWorkspaceResponse,
		UpdateUserRolesInWorkspaceRequest,
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
	redis::expire_tokens_for_user_id,
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

	// List all users with their roles in this workspace
	sub_app.get(
		"/",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::user::LIST,
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
			EveMiddleware::CustomFunction(pin_fn!(
				list_users_with_roles_in_workspace
			)),
		],
	);

	// Add a user to a workspace
	sub_app.post(
		"/:userId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::user::ADD,
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

	sub_app.put(
		"/:userId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::user::UPDATE_ROLES,
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
			EveMiddleware::CustomFunction(pin_fn!(
				update_user_roles_for_workspace
			)),
		],
	);

	sub_app.delete(
		"/:userId",
		[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::workspace::rbac::user::REMOVE,
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
			EveMiddleware::CustomFunction(pin_fn!(remove_user_from_workspace)),
		],
	);

	sub_app
}

async fn list_users_with_roles_in_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)?;

	let users = db::list_all_users_with_roles_in_workspace(
		context.get_database_connection(),
		&workspace_id,
	)
	.await?
	.into_iter()
	.collect();

	context.success(ListUsersWithRolesInWorkspaceResponse { users });
	Ok(context)
}

async fn add_user_to_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)?;

	let user_id = context.get_param(request_keys::USER_ID).unwrap();
	let user_id = Uuid::parse_str(user_id)?;

	let AddUserToWorkspaceRequest { roles, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::add_user_to_workspace_with_roles(
		context.get_database_connection(),
		&user_id,
		&roles,
		&workspace_id,
	)
	.await?;

	context.success(AddUserToWorkspaceResponse {});

	expire_tokens_for_user_id(&mut context.get_state_mut().redis, &user_id)
		.await?;
	Ok(context)
}

async fn update_user_roles_for_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)?;

	let user_id = context.get_param(request_keys::USER_ID).unwrap();
	let user_id = Uuid::parse_str(user_id)?;

	log::trace!(
		"request_id: {} - requested to update user for workspace",
		request_id,
	);

	let UpdateUserRolesInWorkspaceRequest { roles, .. } = context
		.get_body_as()
		.status(400)
		.body(error!(WRONG_PARAMETERS).to_string())?;

	db::remove_user_roles_from_workspace(
		context.get_database_connection(),
		&user_id,
		&workspace_id,
	)
	.await?;
	db::add_user_to_workspace_with_roles(
		context.get_database_connection(),
		&user_id,
		&roles,
		&workspace_id,
	)
	.await?;

	context.success(AddUserToWorkspaceResponse {});

	expire_tokens_for_user_id(&mut context.get_state_mut().redis, &user_id)
		.await?;

	Ok(context)
}

async fn remove_user_from_workspace(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	let request_id = Uuid::new_v4();

	let workspace_id = context.get_param(request_keys::WORKSPACE_ID).unwrap();
	let workspace_id = Uuid::parse_str(workspace_id)?;

	let user_id = context.get_param(request_keys::USER_ID).unwrap();
	let user_id = Uuid::parse_str(user_id)?;

	log::trace!(
		"request_id: {} - requested to remove user - {} from workspace",
		request_id,
		user_id
	);

	db::remove_user_roles_from_workspace(
		context.get_database_connection(),
		&user_id,
		&workspace_id,
	)
	.await?;

	log::trace!(
		"request_id: {} - removed user - {} from workspace",
		request_id,
		user_id
	);

	context.success(RemoveUserFromWorkspaceResponse {});

	expire_tokens_for_user_id(&mut context.get_state_mut().redis, &user_id)
		.await?;
	Ok(context)
}
