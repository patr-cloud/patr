use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
use axum::http::StatusCode;
use models::api::user::*;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to preview a workspace invite before accepting. Verifies the
/// invite token (so only someone with the link can see it) and returns the
/// workspace name for the confirmation screen. Does not consume the invite or
/// check email ownership — that happens on accept.
pub async fn preview_workspace_invite(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: PreviewWorkspaceInvitePath,
				query: (),
				headers:
					PreviewWorkspaceInviteRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: PreviewWorkspaceInviteRequestProcessed { invite_id, token },
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, PreviewWorkspaceInviteRequest>,
) -> Result<AppResponse<PreviewWorkspaceInviteRequest>, ErrorType> {
	info!("Previewing invite `{invite_id}`");

	let now = OffsetDateTime::now_utc();

	let Some(invite) = query!(
		r#"
		SELECT
			workspace_user_invite.token_hash,
			workspace_user_invite.token_expiry,
			workspace.name::TEXT AS "workspace_name!"
		FROM
			workspace_user_invite
		INNER JOIN
			workspace
		ON
			workspace.id = workspace_user_invite.workspace_id
		WHERE
			workspace_user_invite.id = $1;
		"#,
		invite_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::InviteNotFound);
	};

	if invite.token_expiry <= now {
		return Err(ErrorType::InviteExpired);
	}

	let parsed_hash = PasswordHash::new(&invite.token_hash)
		.inspect_err(|err| {
			error!("Error parsing stored invite token hash: `{err}`");
		})
		.map_err(ErrorType::server_error)?;

	// A read-only check: don't count attempts here, the accept endpoint does the
	// brute-force gating. The token keyspace is large enough that the rate
	// limiter is sufficient protection for this preview.
	let token_valid = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{err}`");
	})
	.map_err(ErrorType::server_error)?
	.verify_password(token.as_bytes(), &parsed_hash)
	.is_ok();

	if !token_valid {
		return Err(ErrorType::InviteNotFound);
	}

	AppResponse::builder()
		.body(PreviewWorkspaceInviteResponse {
			workspace_name: invite.workspace_name,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
