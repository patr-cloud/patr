use axum::http::StatusCode;
use models::api::workspace::rbac::role::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to update a role in a workspace. This will update the name,
/// description, and permissions of the role.
pub async fn update_role(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateRolePath {
					role_id,
					workspace_id,
				},
				query: (),
				headers: UpdateRoleRequestHeaders {
					authorization: _,
					user_agent: _,
				},
				body:
					UpdateRoleRequestProcessed {
						role:
							RoleProcessed {
								name,
								description,
								is_immutable: _,
							},
						permissions,
					},
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateRoleRequest>,
) -> Result<AppResponse<UpdateRoleRequest>, ErrorType> {
	info!("Updating role: {}", role_id);

	if permissions.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	// Seeded default roles are read-only.
	let is_immutable = query!(
		r#"
		SELECT
			is_immutable
		FROM
			role
		WHERE
			id = $1 AND
			workspace_id = $2;
		"#,
		role_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::RoleDoesNotExist)?
	.is_immutable;

	if is_immutable {
		return Err(ErrorType::RoleIsImmutable);
	}

	let rows_updated = query!(
		r#"
		UPDATE
			role
		SET
			name = $1,
			description = $2
		WHERE
			id = $3 AND
			workspace_id = $4;
		"#,
		&*name,
		&*description,
		role_id as _,
		workspace_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::RoleAlreadyExists,
		err => ErrorType::server_error(err),
	})?
	.rows_affected();

	if rows_updated == 0 {
		return Err(ErrorType::RoleDoesNotExist);
	}

	trace!("Role updated");

	query!(
		r#"
		DELETE FROM
			role_permission
		WHERE
			role_id = $1;
		"#,
		role_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Bindings are untouched: a role edit changes what the role grants, not
	// where anyone holds it.
	query!(
		r#"
		INSERT INTO
			role_permission(role_id, permission_id)
		SELECT
			$1,
			UNNEST($2::UUID[]);
		"#,
		role_id as _,
		&permissions.into_iter().collect::<Vec<_>>() as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			// Wrong permission ID
			ErrorType::WrongParameters
		}
		other => ErrorType::server_error(other),
	})?;

	trace!("Role permissions replaced");

	redis
		.setex(
			redis::keys::workspace_id_revocation_timestamp(&workspace_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	trace!("Revocation timestamp set");

	AppResponse::builder()
		.body(UpdateRoleResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
