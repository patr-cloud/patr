use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;

use crate::prelude::*;

/// The handler to revoke a pending workspace invite. Requires the permission to
/// modify roles. Once revoked, the invite link no longer works.
pub async fn revoke_workspace_invite(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: RevokeWorkspaceInvitePath {
					workspace_id,
					invite_id,
				},
				query: (),
				headers:
					RevokeWorkspaceInviteRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RevokeWorkspaceInviteRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, RevokeWorkspaceInviteRequest>,
) -> Result<AppResponse<RevokeWorkspaceInviteRequest>, ErrorType> {
	info!("Revoking invite `{invite_id}` from workspace `{workspace_id}`");

	// Scope the lookup to the workspace so an admin of one workspace cannot
	// touch invites belonging to another.
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

	// Delete the role rows first (no cascade), then the invite itself.
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

	query!(
		r#"
		DELETE FROM
			workspace_user_invite
		WHERE
			id = $1 AND
			workspace_id = $2;
		"#,
		invite_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(RevokeWorkspaceInviteResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
