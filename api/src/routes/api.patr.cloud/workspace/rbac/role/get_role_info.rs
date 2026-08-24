use axum::http::StatusCode;
use models::api::workspace::rbac::role::*;

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
		user_data: _,
		state: _,
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
			workspace_id = $2;
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
			permission_id AS "permission_id!: Uuid"
		FROM
			role_permission
		WHERE
			role_id = $1
		ORDER BY
			permission_id;
		"#,
		role_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| row.permission_id)
	.collect::<Vec<_>>();

	AppResponse::builder()
		.body(GetRoleInfoResponse {
			role: WithId::new(
				role.id,
				WorkspaceRole {
					name: role.name,
					description: role.description,
					is_immutable: role.is_immutable,
				},
			),
			permissions,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
