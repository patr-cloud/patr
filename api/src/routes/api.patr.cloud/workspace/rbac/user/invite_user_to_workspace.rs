use std::{collections::BTreeSet, ops::Add};

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::{api::workspace::rbac::user::*, rbac::PermissionScope};
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

	let roles = roles.into_iter().collect::<BTreeSet<_>>();

	if roles.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

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

	// Re-inviting is surfaced as a conflict rather than silently reissuing.
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

	// One row per (role, scope), straight from the request's grants.
	for grant in &roles {
		if matches!(&grant.scope, PermissionScope::Resources(resources) if resources.is_empty()) {
			return Err(ErrorType::WrongParameters);
		}
		let scope_ids = match &grant.scope {
			PermissionScope::Workspace => vec![workspace_id],
			PermissionScope::Resources(resources) => resources.iter().copied().collect(),
		};

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
				$1, $2, $3, *
			FROM
				UNNEST($4::UUID[]);
			"#,
			invite_id as _,
			workspace_id as _,
			grant.role_id as _,
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

	// Cloud serves the dashboard on `app.`; self-hosted off the base domain.
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
