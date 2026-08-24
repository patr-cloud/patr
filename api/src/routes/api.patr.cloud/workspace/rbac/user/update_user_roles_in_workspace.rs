use std::collections::BTreeSet;

use axum::http::StatusCode;
use models::api::workspace::rbac::user::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to update a user's roles in a workspace. This requires the user
/// who is sending the request to have the permission to update roles in the
/// workspace.
pub async fn update_user_roles_in_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateUserRolesInWorkspacePath {
					workspace_id,
					user_id,
				},
				query: (),
				headers:
					UpdateUserRolesInWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: UpdateUserRolesInWorkspaceRequestProcessed { roles },
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateUserRolesInWorkspaceRequest>,
) -> Result<AppResponse<UpdateUserRolesInWorkspaceRequest>, ErrorType> {
	info!("Updating user `{user_id}`'s roles in workspace `{workspace_id}`");

	let roles = roles.into_iter().collect::<BTreeSet<_>>();

	// Membership is unconditional and independent of role-holding: an empty
	// roles list drops the user's bindings but keeps them a member. Removal
	// from the workspace is RemoveUserFromWorkspace's job.
	query!(
		r#"
		INSERT INTO
			workspace_user(user_id, workspace_id)
		VALUES
			($1, $2)
		ON CONFLICT
			(user_id, workspace_id)
		DO NOTHING;
		"#,
		user_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			ErrorType::UserNotFound
		}
		other => ErrorType::server_error(other),
	})?;

	let actor_id = db::ensure_actor_for_user(&mut **database, &user_id, &workspace_id).await?;

	db::delete_bindings_for_actor(&mut **database, &actor_id).await?;

	for role_id in &roles {
		let scopes = db::read_role_scopes(&mut **database, &workspace_id, role_id)
			.await?
			.ok_or(ErrorType::RoleDoesNotExist)?;
		db::mint_bindings(
			&mut **database,
			&workspace_id,
			&actor_id,
			role_id,
			&scopes,
			Some(&user_data.id),
		)
		.await?;
	}

	info!("User's roles updated. Setting revocation timestamp");

	redis
		.setex(
			redis::keys::user_id_revocation_timestamp(&user_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	AppResponse::builder()
		.body(UpdateUserRolesInWorkspaceResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
