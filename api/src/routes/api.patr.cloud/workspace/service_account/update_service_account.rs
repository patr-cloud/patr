use axum::http::StatusCode;
use models::api::workspace::service_account::*;
use rustis::commands::StringCommands;

use crate::prelude::*;

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
		query!(
			r#"
			DELETE FROM
				role_binding
			WHERE
				actor_id = $1;
			"#,
			service_account_id as _,
		)
		.execute(&mut **database)
		.await?;

		for grant in role_bindings {
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
				VALUES
					(GEN_RANDOM_UUID(), $1, $2, $3, $4, NOW(), $5);
				"#,
				workspace_id as _,
				service_account_id as _,
				grant.role_id as _,
				grant.resource_id as _,
				user_data.id as _,
			)
			.execute(&mut **database)
			.await?;
		}

		// Invalidate cached permissions
		redis
			.setex(
				redis::keys::user_id_revocation_timestamp(&service_account_id),
				constants::CACHED_PERMISSIONS_VALIDITY
					.whole_seconds()
					.unsigned_abs(),
				time::OffsetDateTime::now_utc()
					.unix_timestamp_nanos()
					.to_string(),
			)
			.await?;
	}

	AppResponse::builder()
		.body(UpdateServiceAccountResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
