use std::ops::Add;

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;
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
		user_data,
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
		invite_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::InviteNotFound);
	};

	let now = OffsetDateTime::now_utc();

	let token = if cfg!(debug_assertions) {
		constants::WORKSPACE_INVITE_DEBUG_TOKEN.to_string()
	} else {
		format!("{}{}", Uuid::new_v4(), Uuid::new_v4())
	};

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
			invite_attempts = 0,
			invited_by = $3,
			created = $4
		WHERE
			id = $5;
		"#,
		token_hash,
		token_expiry,
		user_data.id as _,
		now,
		invite_id as _,
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
		workspace_id as _,
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
		.body(ResendWorkspaceInviteResponse { accept_url })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
