use axum::http::StatusCode;
use models::api::workspace::runner::*;

use crate::{models::permissions, prelude::*};

/// Renames a runner. The runner process and its Cloudflare entries are keyed
/// by ID, so nothing needs to be told about the change, but the runner's cached
/// auth data holds its name and is marked stale.
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
		redis,
		client_ip: _,
		actor_data: _,
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
		runner_id,
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

	permissions::mark_actor_stale(redis, &runner_id).await?;

	AppResponse::builder()
		.body(UpdateRunnerResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
