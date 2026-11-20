use std::collections::{BTreeMap, BTreeSet};

use axum::http::StatusCode;
use models::{api::workspace::service_account::*, rbac::WorkspacePermission};

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
						service_account: ServiceAccountProcessed { name, description },
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
		service_account_id,
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
		service_account_id,
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

	// Joined to the permissions these bindings grant, to check the caller holds them.
	let granted = query!(
		r#"
		WITH inserted AS (
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
			DO NOTHING
			RETURNING
				role_id,
				scope_id
		)
		SELECT
			role_permission.permission_id AS "permission_id!",
			inserted.scope_id AS "scope_id!"
		FROM
			inserted
		INNER JOIN
			role_permission
		ON
			role_permission.role_id = inserted.role_id;
		"#,
		workspace_id,
		service_account_id,
		&role_bindings
			.iter()
			.map(|grant| grant.role_id)
			.collect::<Vec<_>>() as _,
		&role_bindings
			.iter()
			.map(|grant| grant.resource_id)
			.collect::<Vec<_>>() as _,
		actor_data.id,
	)
	.fetch_all(&mut **database)
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

	if !actor_data
		.permissions
		.get(&workspace_id)
		.is_some_and(|held| {
			held.is_superset_of(
				&WorkspacePermission::Member {
					permissions: granted
						.iter()
						.fold(BTreeMap::new(), |mut permissions, row| {
							permissions
								.entry(row.permission_id)
								.or_insert_with(BTreeSet::new)
								.insert(row.scope_id);
							permissions
						}),
				},
				workspace_id,
			)
		}) {
		return Err(ErrorType::GrantExceedsPermissions);
	}

	// Invalidate cached permissions
	permissions::mark_actor_stale(redis, &service_account_id).await?;

	AppResponse::builder()
		.body(UpdateServiceAccountResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
