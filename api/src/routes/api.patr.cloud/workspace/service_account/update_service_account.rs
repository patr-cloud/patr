use axum::http::StatusCode;
use models::api::workspace::service_account::*;

use crate::{models::permissions, prelude::*};

pub async fn update_service_account(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateServiceAccountPath {
					workspace_id,
					service_account_id,
				},
				query: (),
				headers:
					UpdateServiceAccountRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					UpdateServiceAccountRequestProcessed {
						name,
						description,
						role_bindings,
					},
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateServiceAccountRequest>,
) -> Result<AppResponse<UpdateServiceAccountRequest>, ErrorType> {
	query!(
		r#"
		UPDATE
			service_account
		SET
			name = COALESCE($1, name),
			description = COALESCE($2, description)
		WHERE
			id = $3 AND
			deleted IS NULL;
		"#,
		name.as_deref(),
		description.as_deref(),
		service_account_id as _,
	)
	.execute(&mut **database)
	.await?;

	if let Some(role_bindings) = &role_bindings {
		// Diff against what is requested rather than wipe and reinsert, so a
		// grant that stays keeps its id and who created it.
		let (role_ids, scope_ids) = role_bindings
			.iter()
			.map(|grant| (grant.role_id, grant.resource_id))
			.collect::<(Vec<_>, Vec<_>)>();

		query!(
			r#"
			DELETE FROM
				role_binding
			WHERE
				actor_id = $1 AND
				(role_id, scope_id) NOT IN (
					SELECT
						role_id,
						scope_id
					FROM
						UNNEST($2::UUID[], $3::UUID[]) AS requested(role_id, scope_id)
				);
			"#,
			service_account_id as _,
			&role_ids as _,
			&scope_ids as _,
		)
		.execute(&mut **database)
		.await?;

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
				GEN_RANDOM_UUID(),
				$1,
				$2,
				requested.role_id,
				requested.scope_id,
				NOW(),
				$5
			FROM
				UNNEST($3::UUID[], $4::UUID[]) AS requested(role_id, scope_id)
			ON CONFLICT
				(actor_id, role_id, scope_id)
			DO NOTHING;
			"#,
			workspace_id as _,
			service_account_id as _,
			&role_ids as _,
			&scope_ids as _,
			user_data.id as _,
		)
		.execute(&mut **database)
		.await
		.map_err(|err| match err {
			sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
				match db_err.constraint() {
					Some("role_binding_fk_role_id_workspace_id") => ErrorType::RoleDoesNotExist,
					Some("role_binding_fk_scope_id_workspace_id") => {
						ErrorType::ResourceDoesNotExist
					}
					_ => ErrorType::server_error(sqlx::Error::Database(db_err)),
				}
			}
			other => ErrorType::server_error(other),
		})?;

		// Invalidate cached permissions
		permissions::mark_actor_stale(redis, &service_account_id).await?;
	}

	AppResponse::builder()
		.body(UpdateServiceAccountResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
