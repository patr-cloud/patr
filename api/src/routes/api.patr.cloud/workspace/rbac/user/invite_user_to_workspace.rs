use std::ops::Add;

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to invite a user, by email, to a workspace. Requires the caller
/// to have the permission to modify roles in the workspace. Creates (or
/// refreshes) a pending invite and emails the invitee a link to accept it.
pub async fn invite_user_to_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: InviteUserToWorkspacePath { workspace_id },
				query: (),
				headers:
					InviteUserToWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: InviteUserToWorkspaceRequestProcessed { email, roles },
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		mut state,
	}: AuthenticatedAppRequest<'_, InviteUserToWorkspaceRequest>,
) -> Result<AppResponse<InviteUserToWorkspaceRequest>, ErrorType> {
	info!("Inviting `{email}` to workspace `{workspace_id}`");

	// An invite with no roles would add a member with no permissions (and in
	// fact no `workspace_user` rows at all), so reject it.
	if roles.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	// If the email already belongs to a member (or the owner) of this
	// workspace, there is nothing to invite.
	let already_member = query!(
		r#"
		SELECT
			(
				EXISTS(
					SELECT
						1
					FROM
						user_email
					INNER JOIN
						workspace_user
					ON
						workspace_user.user_id = user_email.user_id
					WHERE
						user_email.email = $1 AND
						workspace_user.workspace_id = $2
				) OR EXISTS(
					SELECT
						1
					FROM
						user_email
					INNER JOIN
						workspace
					ON
						workspace.super_admin_id = user_email.user_id
					WHERE
						user_email.email = $1 AND
						workspace.id = $2
				)
			) AS "already_member!: bool";
		"#,
		email.as_ref(),
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.already_member;

	if already_member {
		return Err(ErrorType::UserAlreadyMember);
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

	// Create the invite. A pending invite for the same email already existing
	// is surfaced as InviteAlreadyExists so the caller edits or revokes it
	// instead of silently resetting the token and resending.
	let invite_id = query!(
		r#"
		INSERT INTO
			workspace_user_invite(
				id,
				workspace_id,
				email,
				token_hash,
				token_expiry,
				invited_by,
				created
			)
		VALUES
			($1, $2, $3, $4, $5, $6, $7)
		RETURNING id AS "id: Uuid";
		"#,
		Uuid::new_v4() as _,
		workspace_id as _,
		email.as_ref(),
		token_hash,
		token_expiry,
		user_data.id as _,
		now,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
			ErrorType::InviteAlreadyExists
		}
		other => ErrorType::server_error(other),
	})?
	.id;

	// Only roles that actually belong to this workspace are inserted. If any
	// requested role is missing or owned by another workspace, fewer rows land
	// than requested — surface that as RoleDoesNotExist (rolls back the invite).
	let inserted = query!(
		r#"
		INSERT INTO
			workspace_user_invite_role(
				invite_id,
				role_id
			)
		SELECT
			$1,
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

	if inserted != roles.len() as u64 {
		return Err(ErrorType::RoleDoesNotExist);
	}

	let workspace_name = query!(
		r#"
		SELECT
			name::TEXT AS "name!"
		FROM
			workspace
		WHERE
			id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut **database)
	.await?
	.name;

	info!("Invite `{invite_id}` created. Sending invite email");

	// The link that goes in the email, also returned once so the caller can
	// offer a "copy link" affordance. Cloud serves the dashboard on the `app.`
	// subdomain; self-hosted path-routes it off the base domain itself.
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
			email.into_owned(),
			WorkspaceInviteEmail {
				workspace_name,
				invited_by: format!("{} {}", user_data.first_name, user_data.last_name),
				accept_url: accept_url.clone(),
				expiry: constants::WORKSPACE_INVITE_VALIDITY.to_string(),
			},
		)
		.await
		.inspect_err(|err| {
			error!("Error enqueuing invite email: `{err}`");
		})?;

	AppResponse::builder()
		.body(InviteUserToWorkspaceResponse {
			id: WithId::from(invite_id),
			accept_url,
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
