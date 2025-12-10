use axum::http::StatusCode;
use models::{api::workspace::runner::*, prelude::*};

use crate::prelude::*;

pub async fn remove_runner_from_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteRunnerPath {
					workspace_id: _,
					runner_id,
				},
				query: (),
				headers:
					DeleteRunnerRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteRunnerRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, DeleteRunnerRequest>,
) -> Result<AppResponse<DeleteRunnerRequest>, ErrorType> {
	info!("Deleting runner `{}`", runner_id);

	query!(
		r#"
		DELETE FROM
			runner
		WHERE
			id = $1;
		"#,
		runner_id as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(dbe) if dbe.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => err.into(),
	})?;

	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		runner_id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(DeleteRunnerResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
