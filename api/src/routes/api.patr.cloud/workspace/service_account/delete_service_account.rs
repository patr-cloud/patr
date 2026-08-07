use axum::http::StatusCode;
use models::api::workspace::service_account::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn delete_service_account(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					DeleteServiceAccountPath {
						workspace_id: _,
						service_account_id,
					},
				query: (),
				headers:
					DeleteServiceAccountRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteServiceAccountRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, DeleteServiceAccountRequest>,
) -> Result<AppResponse<DeleteServiceAccountRequest>, ErrorType> {
	// Remove role assignments
	query!(
		r#"
		DELETE FROM
			workspace_member
		WHERE
			identity_id = $1;
		"#,
		service_account_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Hard-delete the service account row
	// The `identity` and `credential` rows deliberately outlive the service
	// account: audit entries reference the credential, and an audit trail whose
	// actor has vanished is worthless. Authentication still fails for a deleted
	// service account because the token lookup requires `deleted IS NULL`.
	query!(
		r#"
		DELETE FROM
			service_account
		WHERE
			id = $1;
		"#,
		service_account_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Soft-delete the backing resource
	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		service_account_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Invalidate cached permissions
	redis
		.setex(
			redis::keys::user_id_revocation_timestamp(&service_account_id),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await?;

	AppResponse::builder()
		.body(DeleteServiceAccountResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
