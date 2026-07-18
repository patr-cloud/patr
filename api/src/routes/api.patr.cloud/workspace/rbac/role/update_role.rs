use axum::http::StatusCode;
use models::{
	api::workspace::rbac::role::*,
	rbac::{ResourcePermissionType, ResourcePermissionTypeDiscriminant},
};
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to update a role in a workspace. This will update the name,
/// description, and permissions of the role.
pub async fn update_role(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateRolePath {
					role_id,
					workspace_id,
				},
				query: (),
				headers: UpdateRoleRequestHeaders {
					authorization: _,
					user_agent: _,
				},
				body:
					UpdateRoleRequestProcessed {
						name,
						description,
						permissions,
					},
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateRoleRequest>,
) -> Result<AppResponse<UpdateRoleRequest>, ErrorType> {
	info!("Updating role: {}", role_id);

	if permissions.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	let rows_updated = query!(
		r#"
		UPDATE
			role
		SET
			name = $1,
			description = $2
		WHERE
			id = $3 AND
			owner_id = $4;
		"#,
		&*name,
		&*description,
		role_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::RoleAlreadyExists,
		err => ErrorType::server_error(err),
	})?
	.rows_affected();

	if rows_updated == 0 {
		return Err(ErrorType::RoleDoesNotExist);
	}

	trace!("Role updated");

	query!(
		r#"
			DELETE FROM
				role_resource_permissions_include
			WHERE
				role_id = $1;
			"#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted all the included permissions");

	query!(
		r#"
			DELETE FROM
				role_resource_permissions_exclude
			WHERE
				role_id = $1;
			"#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted all the excluded permissions");

	query!(
		r#"
			DELETE FROM
				role_resource_permissions_type
			WHERE
				role_id = $1;
			"#,
		role_id as _
	)
	.execute(&mut **database)
	.await?;

	trace!("Role permissions deleted");

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

	trace!("Role permissions inserted");

	redis
		.setex(
			redis::keys::workspace_id_revocation_timestamp(&workspace_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	trace!("Revocation timestamp set");

	AppResponse::builder()
		.body(UpdateRoleResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
