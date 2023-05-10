use api_models::models::prelude::*;
use axum::{extract::State, Router};
use chrono::{Duration, Utc};

use crate::{
	app::App,
	db,
	models::rbac::permissions,
	prelude::*,
	redis::revoke_user_tokens_created_before_timestamp,
	service::get_access_token_expiry,
	utils::Error,
};

pub fn create_sub_app(app: &App) -> Router<App> {
	Router::new()
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::roles::LIST,
				|ListAllRolesPath { workspace_id }, app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			list_all_roles,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::roles::CREATE,
				|CreateNewRolePath { workspace_id }, app, request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_resource_by_id(&mut connection, &workspace_id)
						.await?;
				},
			),
			app.clone(),
			create_role,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::roles::LIST,
				|GetRoleDetailsPath {
				     workspace_id,
				     role_id,
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
			get_role_details,
		)
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
			list_users_with_role_in_workspace,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::roles::EDIT,
				|UpdateRolePath {
				     workspace_id,
				     role_id,
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_role_by_id(&mut connection, &role_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			update_role,
		)
		.mount_protected_dto(
			ResourceTokenAuthenticator::new(
				permissions::workspace::rbac::roles::DELETE,
				|DeleteRolePath {
				     workspace_id,
				     role_id,
				 },
				 app,
				 request| async {
					let mut connection = request
						.extensions_mut()
						.get_mut::<Connection>()
						.ok_or_else(|| ErrorType::internal_error());

					db::get_role_by_id(&mut connection, &role_id)
						.await?
						.filter(|value| value.owner_id == workspace_id)
				},
			),
			app.clone(),
			delete_role,
		)
}

async fn list_all_roles(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListAllRolesPath { workspace_id },
		query: (),
		body: (),
	}: DecodedRequest<ListAllRolesRequest>,
) -> Result<ListAllRolesResponse, Error> {
	let roles = db::get_all_roles_in_workspace(&mut connection, &workspace_id)
		.await?
		.into_iter()
		.map(|role| Role {
			id: role.id,
			name: role.name,
			description: role.description,
		})
		.collect::<Vec<_>>();

	Ok(ListAllRolesResponse { roles })
}

async fn get_role_details(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: GetRoleDetailsPath {
			workspace_id,
			role_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<GetRoleDetailsRequest>,
) -> Result<GetRoleDetailsResponse, Error> {
	// Check if the role exists
	let role = db::get_role_by_id(&mut connection, &role_id)
		.await?
		.filter(|role| role.owner_id == workspace_id)
		.ok_or_else(|| ErrorType::NotFound)?;

	let resource_permissions =
		db::get_permissions_on_resources_for_role(&mut connection, &role_id)
			.await?
			.into_iter()
			.map(|(key, value)| {
				(
					key,
					value.into_iter().map(|permission| permission.id).collect(),
				)
			})
			.collect();
	let resource_type_permissions =
		db::get_permissions_on_resource_types_for_role(
			&mut connection,
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

	Ok(GetRoleDetailsResponse {
		role: Role {
			id: role.id,
			name: role.name,
			description: role.description,
		},
		resource_permissions,
		resource_type_permissions,
	})
}

async fn create_role(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: CreateNewRolePath { workspace_id },
		query: (),
		body:
			CreateNewRoleRequest {
				name,
				description,
				resource_permissions,
				resource_type_permissions,
			},
	}: DecodedRequest<CreateNewRoleRequest>,
) -> Result<CreateNewRoleResponse, Error> {
	let role_id = db::generate_new_role_id(&mut connection).await?;

	db::create_role(
		&mut connection,
		&role_id,
		&name.trim(),
		&description,
		&workspace_id,
	)
	.await?;
	db::insert_resource_permissions_for_role(
		&mut connection,
		&role_id,
		&resource_permissions,
	)
	.await?;
	db::insert_resource_type_permissions_for_role(
		&mut connection,
		&role_id,
		&resource_type_permissions,
	)
	.await?;

	Ok(CreateNewRoleResponse { id: role_id })
}

async fn list_users_with_role_in_workspace(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: ListUsersForRolePath {
			workspace_id,
			role_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<ListUsersForRoleRequest>,
) -> Result<ListUsersForRoleResponse, Error> {
	db::get_role_by_id(&mut connection, &role_id)
		.await?
		.ok_or_else(|| ErrorType::NotFound)?;

	let users = db::list_all_users_for_role_in_workspace(
		&mut connection,
		&workspace_id,
		&role_id,
	)
	.await?;

	Ok(ListUsersForRoleResponse { users })
}

async fn update_role(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: UpdateRolePath {
			workspace_id,
			role_id,
		},
		query: (),
		body:
			UpdateRoleRequest {
				name,
				description,
				resource_permissions,
				resource_type_permissions,
			},
	}: DecodedRequest<UpdateRoleRequest>,
) -> Result<(), Error> {
	db::update_role_name_and_description(
		&mut connection,
		&role_id,
		name.as_deref(),
		description.as_deref(),
	)
	.await?;

	let associated_users =
		db::get_all_users_with_role(&mut connection, &role_id).await?;

	if let Some((resource_permissions, resource_type_permissions)) =
		resource_permissions.zip(resource_type_permissions)
	{
		db::remove_all_permissions_for_role(&mut connection, &role_id).await?;
		db::insert_resource_permissions_for_role(
			&mut connection,
			&role_id,
			&resource_permissions,
		)
		.await?;
		db::insert_resource_type_permissions_for_role(
			&mut connection,
			&role_id,
			&resource_type_permissions,
		)
		.await?;
	}

	let ttl = get_access_token_expiry() + Duration::hours(2); // 2 hrs buffer time
	for user in associated_users {
		revoke_user_tokens_created_before_timestamp(
			config.redis,
			&user.id,
			&Utc::now(),
			Some(&ttl),
		)
		.await?;
	}

	Ok({})
}

async fn delete_role(
	mut connection: Connection,
	State(config): State<Config>,
	DecodedRequest {
		path: DeleteRolePath {
			workspace_id,
			role_id,
		},
		query: (),
		body: (),
	}: DecodedRequest<DeleteRoleRequest>,
) -> Result<(), Error> {
	let associated_users =
		db::get_all_users_with_role(&mut connection, &role_id).await?;

	if !associated_users.is_empty() {
		return Err(ErrorType::ResourceInUse);
	}

	// Delete role
	db::delete_role(&mut connection, &role_id).await?;

	Ok(())
}
