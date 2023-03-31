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
use axum::Router;
use chrono::{Duration, Utc};
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
pub fn create_sub_route(app: &App) -> Router {
	let router = Router::new();

	// All routes have PlainTokenAuthenticator middleware

	// List all users with their roles in this workspace
	router.route("/", get(list_users_with_roles_in_workspace));

	// Add a user to a workspace
	router.route("/:userId", post(add_user_to_workspace));

	router.route("/:userId", put(update_user_roles_for_workspace));

	router.route("/:userId", delete(remove_user_from_workspace));

	router
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

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	revoke_user_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&user_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	context.success(AddUserToWorkspaceResponse {});
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

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	revoke_user_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&user_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	context.success(AddUserToWorkspaceResponse {});
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

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	revoke_user_tokens_created_before_timestamp(
		context.get_redis_connection(),
		&user_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	context.success(RemoveUserFromWorkspaceResponse {});
	Ok(context)
}
