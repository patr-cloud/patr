use api_models::{
	models::prelude::*,
	utils::{DecodedRequest, DtoRequestExt, Uuid},
	ErrorType,
};
use axum::{extract::State, Extension, Router};
use chrono::{Duration, Utc};
use sqlx::Connection;

use crate::{
	app::App,
	db,
	models::{rbac::permissions, UserAuthenticationData},
	prelude::*,
	redis::revoke_user_tokens_created_before_timestamp,
	service::get_access_token_expiry,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::user::LIST,
				|ListUsersWithRolesInWorkspacePath { workspace_id },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			list_users_with_roles_in_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::user::ADD,
				|AddUserToWorkspacePath {
				     workspace_id,
				     user_id,
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			add_user_to_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::user::UPDATE_ROLES,
				|UpdateUserRolesInWorkspacePath {
				     workspace_id,
				     user_id,
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			update_user_roles_for_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::user::REMOVE,
				|RemoveUserFromWorkspacePath {
				     workspace_id,
				     user_id,
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			remove_user_from_workspace,
		)
}

async fn list_users_with_roles_in_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(access_token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: ListUsersWithRolesInWorkspacePath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListUsersWithRolesInWorkspaceRequest>,
) -> Result<ListUsersWithRolesInWorkspaceResponse, Error> {
	let users = db::list_all_users_with_roles_in_workspace(
		&mut connection,
		&workspace_id,
	)
	.await?
	.into_iter()
	.collect();

	Ok(ListUsersWithRolesInWorkspaceResponse { users })
}

async fn add_user_to_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(access_token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: AddUserToWorkspacePath {
			workspace_id,
			user_id,
		},
		query: (),
		body: AddUserToWorkspaceRequest { roles },
	}: DecodedRequest<AddUserToWorkspaceRequest>,
) -> Result<(), Error> {
	db::add_user_to_workspace_with_roles(
		&mut connection,
		&user_id,
		&roles,
		&workspace_id,
	)
	.await?;

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	revoke_user_tokens_created_before_timestamp(
		config.redis,
		&user_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	Ok(())
}

async fn update_user_roles_for_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(access_token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: UpdateUserRolesInWorkspacePath {
			workspace_id,
			user_id,
		},
		query: (),
		body: UpdateUserRolesInWorkspaceRequest { roles },
	}: DecodedRequest<UpdateUserRolesInWorkspaceRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - requested to update user for workspace",
		request_id,
	);

	db::remove_user_roles_from_workspace(
		&mut connection,
		&user_id,
		&workspace_id,
	)
	.await?;
	db::add_user_to_workspace_with_roles(
		&mut connection,
		&user_id,
		&roles,
		&workspace_id,
	)
	.await?;

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	revoke_user_tokens_created_before_timestamp(
		config.redis,
		&user_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	Ok(())
}

async fn remove_user_from_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	Extension(access_token_data): Extension<UserAuthenticationData>,
	DecodedRequest {
		path: RemoveUserFromWorkspacePath {
			workspace_id,
			user_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<RemoveUserFromWorkspaceRequest>,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();

	log::trace!(
		"request_id: {} - requested to remove user - {} from workspace",
		request_id,
		user_id
	);

	db::remove_user_roles_from_workspace(
		&mut connection,
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
		config.redis,
		&user_id,
		&Utc::now(),
		Some(&ttl),
	)
	.await?;

	Ok(())
}
