use std::collections::BTreeSet;

use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;

use crate::prelude::*;

/// The handler to update the roles a pending invite will grant once accepted.
/// Requires the permission to modify roles. The invite token and email are left
/// untouched — only the granted role set changes.
pub async fn update_workspace_invite_roles(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateWorkspaceInviteRolesPath {
					workspace_id,
					invite_id,
				},
				query: (),
				headers:
					UpdateWorkspaceInviteRolesRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: UpdateWorkspaceInviteRolesRequestProcessed { roles },
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateWorkspaceInviteRolesRequest>,
) -> Result<AppResponse<UpdateWorkspaceInviteRolesRequest>, ErrorType> {
	info!("Updating roles for invite `{invite_id}` in workspace `{workspace_id}`");

	let roles = roles.into_iter().collect::<BTreeSet<_>>();

	if roles.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	let exists = query!(
		r#"
		SELECT EXISTS(
			SELECT
				1
			FROM
				workspace_user_invite
			WHERE
				id = $1 AND
				workspace_id = $2
		) AS "exists!: bool";
		"#,
		invite_id as _,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.exists;

	if !exists {
		return Err(ErrorType::InviteNotFound);
	}

	// The request rolls back on failure, so the invite keeps its old roles.
	query!(
		r#"
		DELETE FROM
			workspace_user_invite_role
		WHERE
			invite_id = $1;
		"#,
		invite_id as _,
	)
	.execute(&mut **database)
	.await?;

	// One row per grant, straight from the request.
	let role_ids = roles.iter().map(|grant| grant.role_id).collect::<Vec<_>>();
	let scope_ids = roles
		.iter()
		.map(|grant| grant.resource_id)
		.collect::<Vec<_>>();

	query!(
		r#"
		INSERT INTO
			workspace_user_invite_role(
				invite_id,
				workspace_id,
				role_id,
				scope_id
			)
		SELECT
			$1,
			$2,
			UNNEST($3::UUID[]),
			UNNEST($4::UUID[]);
		"#,
		invite_id as _,
		workspace_id as _,
		&role_ids as _,
		&scope_ids as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			match db_err.constraint() {
				Some("workspace_user_invite_role_fk_scope_id_workspace_id") => {
					ErrorType::ResourceDoesNotExist
				}
				_ => ErrorType::RoleDoesNotExist,
			}
		}
		other => ErrorType::server_error(other),
	})?;

	AppResponse::builder()
		.body(UpdateWorkspaceInviteRolesResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
