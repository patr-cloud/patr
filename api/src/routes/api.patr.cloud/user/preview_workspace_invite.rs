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

	let success = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{err}`");
	})
	.map_err(ErrorType::server_error)?
	.verify_password(
		token.as_bytes(),
		&PasswordHash::new(&invite.token_hash).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		info!("Error verifying invite token: `{err}`");
	})
	.is_ok();

	if !success {
		return Err(ErrorType::InviteNotFound);
	}

	// Only past a valid token, so that a caller holding nothing but an invite id
	// can't tell an expired invite apart from one that never existed.
	if invite.token_expiry <= now {
		return Err(ErrorType::InviteExpired);
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
