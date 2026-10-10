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
						service_account:
							ServiceAccountProcessed {
								name,
								description,
								is_immutable: _,
							},
						role_bindings,
					},
			},
		database,
		redis,
		client_ip: _,
		actor_data,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateServiceAccountRequest>,
) -> Result<AppResponse<UpdateServiceAccountRequest>, ErrorType> {
	// A runner's service account changes only through the runner routes.
	let is_immutable = query!(
		r#"
		SELECT
			is_immutable
		FROM
			service_account
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		service_account_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?
	.is_immutable;

	if is_immutable {
		return Err(ErrorType::ServiceAccountIsImmutable);
	}

	let rows_updated = query!(
		r#"
		UPDATE
			service_account
		SET
			name = $1,
			description = $2
		WHERE
			id = $3 AND
			deleted IS NULL;
		"#,
		name.as_ref(),
		description.as_deref(),
		service_account_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		other => other.into(),
	})?
	.rows_affected();

	if rows_updated == 0 {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	// Diff against what is requested rather than wipe and reinsert, so a
	// grant that stays keeps its id and who created it.
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
		&role_bindings
			.iter()
			.map(|grant| grant.role_id)
			.collect::<Vec<_>>() as _,
		&role_bindings
			.iter()
			.map(|grant| grant.resource_id)
			.collect::<Vec<_>>() as _,
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
		&role_bindings
			.iter()
			.map(|grant| grant.role_id)
			.collect::<Vec<_>>() as _,
		&role_bindings
			.iter()
			.map(|grant| grant.resource_id)
			.collect::<Vec<_>>() as _,
		actor_data.id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
			match db_err.constraint() {
				Some("role_binding_fk_role_id_workspace_id") => ErrorType::RoleDoesNotExist,
				Some("role_binding_fk_scope_id_workspace_id") => ErrorType::ResourceDoesNotExist,
				_ => ErrorType::server_error(sqlx::Error::Database(db_err)),
			}
		}
		other => ErrorType::server_error(other),
	})?;

	// Invalidate cached permissions
	permissions::mark_actor_stale(redis, &service_account_id).await?;

	AppResponse::builder()
		.body(UpdateServiceAccountResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
