use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler for the authenticated user to accept a workspace invite. The
/// `invite_id` and `token` come from the invite email link. The caller must own
/// the email address the invite was sent to, otherwise the invite is rejected.
pub async fn accept_workspace_invite(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: AcceptWorkspaceInvitePath,
				query: (),
				headers:
					AcceptWorkspaceInviteRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: AcceptWorkspaceInviteRequestProcessed { invite_id, token },
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, AcceptWorkspaceInviteRequest>,
) -> Result<AppResponse<AcceptWorkspaceInviteRequest>, ErrorType> {
	info!("User `{}` accepting invite `{invite_id}`", user_data.id);

	let now = OffsetDateTime::now_utc();

	let Some(invite) = query!(
		r#"
		SELECT
			workspace_id AS "workspace_id: Uuid",
			email,
			token_hash,
			token_expiry,
			invite_attempts
		FROM
			workspace_user_invite
		WHERE
			id = $1;
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

	// A locked invite (too many wrong-token attempts) is treated as
	// non-existent so it can't be brute-forced further.
	if invite.invite_attempts >= constants::MAX_WORKSPACE_INVITE_ATTEMPTS {
		return Err(ErrorType::InviteNotFound);
	}

	let parsed_hash = PasswordHash::new(&invite.token_hash)
		.inspect_err(|err| {
			error!("Error parsing stored invite token hash: `{err}`");
		})
		.map_err(ErrorType::server_error)?;

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
		// Count the failed attempt, then return a not-found so we don't leak
		// whether the invite exists. Counted on the pool rather than the request
		// transaction, which the `Err` below rolls back — an increment written
		// there would be discarded with it, leaving the ceiling above
		// permanently unreachable.
		query!(
			r#"
			UPDATE
				workspace_user_invite
			SET
				invite_attempts = invite_attempts + 1
			WHERE
				id = $1;
			"#,
			invite_id as _,
		)
		.execute(&state.database)
		.await?;

		return Err(ErrorType::InviteNotFound);
	}

	// The invite was addressed to an email; only the (verified) owner of that
	// email may accept it, even if a third party somehow has the link.
	let owns_email = query!(
		r#"
		SELECT EXISTS(
			SELECT
				1
			FROM
				user_email
			WHERE
				user_id = $1 AND
				email = $2
		) AS "owns_email!: bool";
		"#,
		user_data.id as _,
		invite.email,
	)
	.fetch_one(&mut **database)
	.await?
	.owns_email;

	if !owns_email {
		return Err(ErrorType::InviteEmailMismatch);
	}

	let workspace_id = invite.workspace_id;

	// Grant the invited roles, re-validating each against the workspace so a
	// role deleted since the invite was sent is simply skipped.
	query!(
		r#"
		INSERT INTO
			workspace_user(
				user_id,
				workspace_id,
				role_id
			)
		SELECT
			$1,
			$2,
			workspace_user_invite_role.role_id
		FROM
			workspace_user_invite_role
		INNER JOIN
			role
		ON
			role.id = workspace_user_invite_role.role_id AND
			role.owner_id = $2
		WHERE
			workspace_user_invite_role.invite_id = $3
		ON CONFLICT
			(user_id, workspace_id, role_id)
		DO NOTHING;
		"#,
		user_data.id as _,
		workspace_id as _,
		invite_id as _,
	)
	.execute(&mut **database)
	.await?;

	// The invite has been consumed — remove it and its role rows.
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
			id = $1;
		"#,
		invite_id as _,
	)
	.execute(&mut **database)
	.await?;

	info!("Invite accepted. Setting revocation timestamp");

	redis
		.setex(
			redis::keys::user_id_revocation_timestamp(&user_data.id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{err}`");
		})?;

	AppResponse::builder()
		.body(AcceptWorkspaceInviteResponse {
			id: WithId::from(workspace_id),
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
