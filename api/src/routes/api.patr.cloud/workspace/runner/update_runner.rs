use axum::http::StatusCode;
use models::api::workspace::runner::*;

use crate::prelude::*;

/// Renames a runner. The name only lives in the `runner` row — the runner
/// process, its Cloudflare entries and the Redis caches are all keyed by ID —
/// so nothing needs to be told about the change.
pub async fn update_runner(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateRunnerPath {
					workspace_id: _,
					runner_id,
				},
				query: (),
				headers: _,
				body: UpdateRunnerRequestProcessed { name },
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, UpdateRunnerRequest>,
) -> Result<AppResponse<UpdateRunnerRequest>, ErrorType> {
	info!("Updating runner `{}`", runner_id);

	let rows_updated = query!(
		r#"
		UPDATE
			runner
		SET
			name = $1
		WHERE
			id = $2 AND
			deleted IS NULL;
		"#,
		name as _,
		runner_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(dbe) if dbe.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		err => err.into(),
	})?
	.rows_affected();

	if rows_updated == 0 {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	AppResponse::builder()
		.body(UpdateRunnerResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
