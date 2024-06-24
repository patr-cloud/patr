use axum::http::StatusCode;
use models::{
	api::workspace::rbac::role::*,
	rbac::{ResourcePermissionType, ResourcePermissionTypeDiscriminant},
};

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
						name,
						description,
						permissions,
					},
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, CreateNewRoleRequest>,
) -> Result<AppResponse<CreateNewRoleRequest>, ErrorType> {
	info!("Creating new role: {} in workspace: {}", name, workspace_id);

	let role_id = query!(
		r#"
		INSERT INTO
			role(
				id,
				owner_id,
				name,
				description
			)
		VALUES
			(
				GENERATE_ROLE_ID(),
				$1,
				$2,
				$3
			)
		RETURNING id;
		"#,
		workspace_id as _,
		name as _,
		description as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::RoleAlreadyExists,
		other => other.into(),
	})?
	.id;

	trace!("Role created. Inserting permissions.");

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
				.await?;
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
				.await?;
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
