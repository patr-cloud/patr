use std::{
	collections::{BTreeMap, BTreeSet},
	ops::Add,
};

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::{api::workspace::rbac::user::*, rbac::WorkspacePermission};
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to resend a pending workspace invite. Requires the permission to
/// modify roles. Regenerates the token (invalidating the old link), refreshes
/// the expiry, resets the attempt counter, and sends the invite email again.
/// The invited roles are left unchanged.
pub async fn resend_workspace_invite(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ResendWorkspaceInvitePath {
					workspace_id,
					invite_id,
				},
				query: (),
				headers:
					ResendWorkspaceInviteRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ResendWorkspaceInviteRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		actor_data,
		mut state,
	}: AuthenticatedAppRequest<'_, ResendWorkspaceInviteRequest>,
) -> Result<AppResponse<ResendWorkspaceInviteRequest>, ErrorType> {
	info!("Resending invite `{invite_id}` in workspace `{workspace_id}`");

	// The invite must exist and belong to this workspace.
	let Some(invite) = query!(
		r#"
		SELECT
			email
		FROM
			workspace_user_invite
		WHERE
			id = $1 AND
			workspace_id = $2;
		"#,
		invite_id,
		workspace_id,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::InviteNotFound);
	};

	// Joined to the permissions these bindings grant, to check the caller holds them.
	let granted = query!(
		r#"
		SELECT
			role_permission.permission_id,
			workspace_user_invite_role.scope_id
		FROM
			workspace_user_invite_role
		INNER JOIN
			role_permission
		ON
			role_permission.role_id = workspace_user_invite_role.role_id
		WHERE
			workspace_user_invite_role.invite_id = $1;
		"#,
		invite_id,
	)
	.fetch_all(&mut **database)
	.await?;

	if !actor_data
		.permissions
		.get(&workspace_id)
		.is_some_and(|held| {
			held.is_superset_of(
				&WorkspacePermission::Member {
					permissions: granted
						.iter()
						.fold(BTreeMap::new(), |mut permissions, row| {
							permissions
								.entry(row.permission_id)
								.or_insert_with(BTreeSet::new)
								.insert(row.scope_id);
							permissions
						}),
				},
				workspace_id,
			)
		}) {
		return Err(ErrorType::GrantExceedsPermissions);
	}

	let now = OffsetDateTime::now_utc();

	// Two v4 UUIDs = 256 bits, non-hyphenated hex. Only its hash is stored, so
	// the `accept_url` below is the one and only time it can be read back.
	let token = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());

	let token_hash = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{err}`");
	})
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(token.as_bytes(), &generate_salt())
	.inspect_err(|err| {
		error!("Error hashing invite token: `{err}`");
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	let token_expiry = now.add(constants::WORKSPACE_INVITE_VALIDITY);

	query!(
		r#"
		UPDATE
			workspace_user_invite
		SET
			token_hash = $1,
			token_expiry = $2,
			invited_by = $3,
			created = $4
		WHERE
			id = $5;
		"#,
		token_hash,
		token_expiry,
		actor_data.id,
		now,
		invite_id,
	)
	.execute(&mut **database)
	.await?;

	let workspace_name = query!(
		r#"
		SELECT
			name::TEXT AS "name!"
		FROM
			workspace
		WHERE
			id = $1;
		"#,
		workspace_id,
	)
	.fetch_one(&mut **database)
	.await?
	.name;

	info!("Invite `{invite_id}` refreshed. Resending invite email");

	// The refreshed link, also returned once so the caller can offer a "copy
	// link" affordance. Cloud serves the dashboard on the `app.` subdomain;
	// self-hosted path-routes it off the base domain itself.
	let base_domain = &state.config.server.base_domain;
	let dashboard_url = if cfg!(feature = "cloud") {
		format!("https://app.{base_domain}")
	} else {
		format!("https://{base_domain}")
	};
	let accept_url = format!("{dashboard_url}/accept-invite?inviteId={invite_id}&token={token}");

	state
		.worker
		.send_email(
			invite.email,
			WorkspaceInviteEmail {
				workspace_name,
				invited_by: actor_data.actor.display_name(),
				accept_url: accept_url.clone(),
				expiry: constants::WORKSPACE_INVITE_VALIDITY.to_string(),
			},
		)
		.await
		.inspect_err(|err| {
			error!("Error enqueuing invite email: `{err}`");
		})?;

	AppResponse::builder()
		.body(ResendWorkspaceInviteResponse { accept_url })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
