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
						roles,
					},
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
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

	if let Some(roles) = &roles {
		query!(
			r#"
			DELETE FROM
				service_account_role
			WHERE
				service_account_id = $1;
			"#,
			service_account_id as _,
		)
		.execute(&mut **database)
		.await?;

		for role_id in roles {
			query!(
				r#"
				INSERT INTO
					service_account_role(
						service_account_id,
						workspace_id,
						role_id
					)
				VALUES
					($1, $2, $3);
				"#,
				service_account_id as _,
				workspace_id as _,
				role_id as _,
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
