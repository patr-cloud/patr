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
			invited_by AS "invited_by: Uuid"
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
		// Reported as a not-found so we don't leak whether the invite exists.
		return Err(ErrorType::InviteNotFound);
	}

	// Only past a valid token, so that a caller holding nothing but an invite id
	// can't tell an expired invite apart from one that never existed.
	if invite.token_expiry <= now {
		return Err(ErrorType::InviteExpired);
	}

	let owns_email = query!(
		r#"
		SELECT EXISTS(
			SELECT
				1
			FROM
				"user"
			WHERE
				id = $1 AND
				email = $2::CITEXT
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

	// Membership is unconditional — an invite whose roles were deleted in
	// the meantime still makes the accepter a (zero-binding) member instead
	// of silently making nobody anything. Re-accepting keeps the actor the
	// member already has, so their existing bindings stay attached.
	let existing_actor = query!(
		r#"
		SELECT
			actor_id AS "id: Uuid"
		FROM
			workspace_user
		WHERE
			user_id = $1 AND
			workspace_id = $2;
		"#,
		user_data.id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| row.id);

	let actor_id = match existing_actor {
		Some(actor_id) => actor_id,
		None => {
			// The actor is the identity bindings hang off, so it is minted
			// first and the membership row points at it.
			let actor_id = query!(
				r#"
				INSERT INTO
					workspace_actor(id, workspace_id, actor_type)
				VALUES
					(gen_random_uuid(), $1, 'user')
				RETURNING id AS "id: Uuid";
				"#,
				workspace_id as _,
			)
			.fetch_one(&mut **database)
			.await?
			.id;

			query!(
				r#"
				INSERT INTO
					workspace_user(user_id, workspace_id, actor_id)
				VALUES
					($1, $2, $3);
				"#,
				user_data.id as _,
				workspace_id as _,
				&actor_id as _,
			)
			.execute(&mut **database)
			.await?;

			actor_id
		}
	};

	// The invite rows already carry concrete scopes; mint bindings directly,
	// attributing the grant to the inviter.
	query!(
		r#"
		INSERT INTO
			role_binding(
				id,
				workspace_id,
				actor_id,
				role_id,
				scope_id,
				created,
				created_by
			)
		SELECT
			gen_random_uuid(),
			ir.workspace_id,
			$1,
			ir.role_id,
			ir.scope_id,
			NOW(),
			$3
		FROM
			workspace_user_invite_role ir
		WHERE
			ir.invite_id = $2
		ON CONFLICT
			(actor_id, role_id, scope_id)
		DO NOTHING;
		"#,
		actor_id as _,
		invite_id as _,
		invite.invited_by as _,
	)
	.execute(&mut **database)
	.await?;

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
