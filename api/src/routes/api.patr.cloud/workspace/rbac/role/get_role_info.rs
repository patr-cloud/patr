use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::{
	api::workspace::rbac::role::*,
	rbac::{ResourcePermissionType, ResourcePermissionTypeDiscriminant},
};

use crate::prelude::*;

/// The handler to get all the details of a role in a workspace. This will
/// return the name, description, and permissions of the role.
pub async fn get_role_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetRoleInfoPath {
					workspace_id,
					role_id,
				},
				query: (),
				headers:
					GetRoleInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetRoleInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetRoleInfoRequest>,
) -> Result<AppResponse<GetRoleInfoRequest>, ErrorType> {
	info!(
		"Listing all the details for the role: {} in workspace: {}",
		role_id, workspace_id
	);

	let role = query!(
		r#"
		SELECT
			*
		FROM
			role
		WHERE
			id = $1 AND
			owner_id = $2;
		"#,
		role_id as _,
		workspace_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::RoleDoesNotExist)?;

	trace!("Basic role details fetched");

	let permissions = query!(
		r#"
		SELECT
			COALESCE(
				include.resource_id,
				exclude.resource_id
			) AS "permission_id!",
			COALESCE(
				role_resource_permissions_type.permission_type,
				role_resource_permissions_type.permission_type
			) AS "permission_type!: ResourcePermissionTypeDiscriminant",
			COALESCE(
				include.resource_id,
				exclude.resource_id
			) AS "resource_id!"
		FROM
			role_resource_permissions_type
		LEFT JOIN
			role_resource_permissions_include include
		ON
			role_resource_permissions_type.permission_id = include.permission_id
		LEFT JOIN
			role_resource_permissions_exclude exclude
		ON
			role_resource_permissions_type.permission_id = exclude.permission_id
		WHERE
			role_resource_permissions_type.role_id = $1;
		"#,
		role_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.fold(BTreeMap::new(), |mut map, row| {
		map.entry(row.permission_id.into())
			.or_insert(match row.permission_type {
				ResourcePermissionTypeDiscriminant::Include => {
					ResourcePermissionType::Include(Default::default())
				}
				ResourcePermissionTypeDiscriminant::Exclude => {
					ResourcePermissionType::Exclude(Default::default())
				}
			})
			.insert(row.resource_id.into());
		map
	});

	AppResponse::builder()
		.body(GetRoleInfoResponse {
			role: WithId::new(
				role.id,
				Role {
					name: role.name,
					description: role.description,
				},
			),
			permissions,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
