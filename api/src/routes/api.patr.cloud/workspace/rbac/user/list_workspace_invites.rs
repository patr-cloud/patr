use std::collections::{BTreeMap, BTreeSet};

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
			workspace_user_invite.token_expiry,
			workspace_user_invite_role.role_id AS "role_id?: Uuid",
			workspace_user_invite_role.scope_id AS "scope_id?: Uuid"
		FROM
			workspace_user_invite
		LEFT JOIN
			workspace_user_invite_role
		ON
			workspace_user_invite_role.invite_id = workspace_user_invite.id
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

	// The LEFT JOIN gives one row per (invite, role, scope), so fold them
	// back into one entry per invite, accumulating per-resource rows of one
	// role into a single grant.
	let invites = rows
		.into_iter()
		.fold(
			BTreeMap::<Uuid, WithId<WorkspaceInvite>>::new(),
			|mut invites, row| {
				let invite = invites.entry(row.id).or_insert_with(|| {
					WithId::new(
						row.id,
						WorkspaceInvite {
							email: row.email.clone(),
							roles: Vec::new(),
							invited_by: row.invited_by,
							created: row.created,
							expiry: row.token_expiry,
						},
					)
				});

				let (Some(role_id), Some(scope_id)) = (row.role_id, row.scope_id) else {
					return invites;
				};

				invite.data.roles.push(RoleGrant {
					role_id,
					resource_id: scope_id,
				});

				invites
			},
		)
		.into_values()
		.collect();

	AppResponse::builder()
		.body(ListWorkspaceInvitesResponse { invites })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
