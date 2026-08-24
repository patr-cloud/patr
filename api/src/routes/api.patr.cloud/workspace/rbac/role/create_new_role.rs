use axum::http::StatusCode;
use models::{
	api::workspace::rbac::role::*,
	rbac::{ResourcePermissionType, ResourcePermissionTypeDiscriminant},
};
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to create a new role in a workspace. This will create a new role
/// with the provided name, description, and permissions. The permissions will
/// determine what a user with the mentioned role can do in the workspace.
pub async fn create_new_role(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateNewRolePath { workspace_id },
				query: (),
				headers:
					CreateNewRoleRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					CreateNewRoleRequestProcessed {
						role: RoleProcessed { name, description },
						permissions,
					},
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, CreateNewRoleRequest>,
) -> Result<AppResponse<CreateNewRoleRequest>, ErrorType> {
	info!("Creating new role: {} in workspace: {}", name, workspace_id);

	if permissions.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	// A binding applies the whole role at one scope, so every permission in a
	// role must carry the same resource set. Non-uniform roles became
	// unrepresentable at the role-binding cutover.
	let mut values = permissions.values();
	let first = values.next();
	if values.any(|value| Some(value) != first) {
		return Err(ErrorType::WrongParameters);
	}

	let description = if description.is_empty() {
		"No description provided".into()
	} else {
		description
	};

	let now = OffsetDateTime::now_utc();

	let role_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				workspace_id,
				created,
				deleted
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'role'),
				$1,
				$2,
				NULL
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
		now as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::RoleAlreadyExists,
		err => ErrorType::server_error(err),
	})?
	.id;

	query!(
		r#"
		INSERT INTO
			role(
				id,
				workspace_id,
				name,
				description
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4
			);
		"#,
		role_id as _,
		workspace_id as _,
		name as _,
		description as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::RoleAlreadyExists,
		err => ErrorType::server_error(err),
	})?;

	trace!("Role created. Inserting permissions.");

	query!(
		r#"
		INSERT INTO
			role_permission(role_id, permission_id)
		SELECT
			$1,
			UNNEST($2::UUID[]);
		"#,
		role_id as _,
		&permissions
			.keys()
			.copied()
			.map(Into::into)
			.collect::<Vec<_>>(),
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			ErrorType::ResourceDoesNotExist
		}
		other => ErrorType::server_error(other),
	})?;

	// The legacy per-permission template rows are still dual-written: until
	// the assignment DTOs carry scopes, they are where a role's scope is
	// read from at assignment time.
	for (permission_id, permission) in permissions {
		let permission_type = ResourcePermissionTypeDiscriminant::from(&permission);
		query!(
			r#"
			INSERT INTO
				role_resource_permissions_type(
					role_id,
					permission_id,
					permission_type
				)
			VALUES
				(
					$1,
					$2,
					$3
				);
			"#,
			role_id as _,
			permission_id as _,
			permission_type as _,
		)
		.execute(&mut **database)
		.await?;
		match permission {
			ResourcePermissionType::Include(resources) => {
				query!(
					r#"
					INSERT INTO
						role_resource_permissions_include(
							role_id,
							permission_id,
							resource_id,
							permission_type
						)
					VALUES
						(
							$1,
							$2,
							UNNEST($3::UUID[]),
							DEFAULT
						);
					"#,
					role_id as _,
					permission_id as _,
					&resources.into_iter().map(|r| r.into()).collect::<Vec<_>>(),
				)
				.execute(&mut **database)
				.await
				.map_err(|err| match err {
					sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
						ErrorType::ResourceDoesNotExist
					}
					other => ErrorType::server_error(other),
				})?;
			}
			ResourcePermissionType::Exclude(resources) => {
				query!(
					r#"
					INSERT INTO
						role_resource_permissions_exclude(
							role_id,
							permission_id,
							resource_id,
							permission_type
						)
					VALUES
						(
							$1,
							$2,
							UNNEST($3::UUID[]),
							DEFAULT
						);
					"#,
					role_id as _,
					permission_id as _,
					&resources.into_iter().map(|r| r.into()).collect::<Vec<_>>(),
				)
				.execute(&mut **database)
				.await
				.map_err(|err| match err {
					sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
						ErrorType::ResourceDoesNotExist
					}
					other => ErrorType::server_error(other),
				})?;
			}
		};
	}

	AppResponse::builder()
		.body(CreateNewRoleResponse {
			id: WithId::from(role_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
