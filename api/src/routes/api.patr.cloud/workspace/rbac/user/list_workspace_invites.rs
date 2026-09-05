use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;

use crate::prelude::*;

/// The handler to list all pending invites for a workspace, along with the
/// roles each invitee will be granted. Requires the permission to view roles.
pub async fn list_workspace_invites(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListWorkspaceInvitesPath { workspace_id },
				query: (),
				headers:
					ListWorkspaceInvitesRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListWorkspaceInvitesRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, ListWorkspaceInvitesRequest>,
) -> Result<AppResponse<ListWorkspaceInvitesRequest>, ErrorType> {
	info!("Listing pending invites for workspace `{workspace_id}`");

	let rows = query!(
		r#"
		SELECT
			workspace_user_invite.id AS "id: Uuid",
			workspace_user_invite.email,
			workspace_user_invite.invited_by AS "invited_by: Uuid",
			workspace_user_invite.created,
			workspace_user_invite.token_expiry
		FROM
			workspace_user_invite
		WHERE
			workspace_user_invite.workspace_id = $1
		ORDER BY
			workspace_user_invite.created DESC,
			workspace_user_invite.id;
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut **database)
	.await?;

	// One flat query for every invite's grants, then attach. Keeping it out of
	// the query above leaves that one row per invite, in the order it asked for.
	let mut grants = query!(
		r#"
		SELECT
			workspace_user_invite_role.invite_id AS "invite_id: Uuid",
			workspace_user_invite_role.role_id AS "role_id: Uuid",
			workspace_user_invite_role.scope_id AS "scope_id: Uuid"
		FROM
			workspace_user_invite_role
		WHERE
			workspace_user_invite_role.invite_id = ANY($1::UUID[]);
		"#,
		&rows.iter().map(|row| row.id).collect::<Vec<_>>() as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.fold(
		BTreeMap::<Uuid, Vec<RoleBindingGrant>>::new(),
		|mut grants, row| {
			grants
				.entry(row.invite_id)
				.or_default()
				.push(RoleBindingGrant {
					role_id: row.role_id,
					resource_id: row.scope_id,
				});
			grants
		},
	);

	let invites = rows
		.into_iter()
		.map(|row| {
			WithId::new(
				row.id,
				WorkspaceInvite {
					email: row.email,
					roles: grants.remove(&row.id).unwrap_or_default(),
					invited_by: row.invited_by,
					created: row.created,
					expiry: row.token_expiry,
				},
			)
		})
		.collect::<Vec<_>>();

	AppResponse::builder()
		.body(ListWorkspaceInvitesResponse { invites })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
