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

	// An invite with no roles would be meaningless (accepting it adds no
	// membership), so reject it.
	if roles.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	// The invite must exist and belong to this workspace.
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

	// Replace the role set. On failure (a role that doesn't belong to the
	// workspace) the whole request rolls back, so the invite keeps its old
	// roles.
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

	let inserted = query!(
		r#"
		INSERT INTO
			workspace_user_invite_role(
				invite_id,
				workspace_id,
				role_id
			)
		SELECT
			$1,
			$3,
			role.id
		FROM
			role
		WHERE
			role.id = ANY($2::UUID[]) AND
			role.owner_id = $3;
		"#,
		invite_id as _,
		roles as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	// Distinct, because the SELECT matches each role once — a repeated id would
	// otherwise land fewer rows than asked for and look like a missing role.
	if inserted != roles.iter().collect::<BTreeSet<_>>().len() as u64 {
		return Err(ErrorType::RoleDoesNotExist);
	}

	AppResponse::builder()
		.body(UpdateWorkspaceInviteRolesResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
