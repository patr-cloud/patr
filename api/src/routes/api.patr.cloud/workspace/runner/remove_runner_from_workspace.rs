use axum::http::StatusCode;
use models::{api::workspace::runner::*, prelude::*};

use crate::prelude::*;

pub async fn remove_runner_from_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteRunnerPath {
					workspace_id,
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
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteRunnerRequest>,
) -> Result<AppResponse<DeleteRunnerRequest>, ErrorType> {
	info!("Deleting runner `{}`", runner_id);

	let runner = query!(
		r#"
		SELECT
			runner.id,
			runner.workspace_id
		FROM
			runner
		INNER JOIN
			resource
		ON
			runner.id = resource.id
		WHERE
			runner.id = $1 AND
			runner.deleted IS NULL AND
			resource.owner_id = $2;
		"#,
		runner_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?;

	if let Some(runner) = runner {
		query!(
			r#"
			SET CONSTRAINTS ALL DEFERRED;
			"#
		)
		.execute(&mut **database)
		.await?;

		query!(
			r#"
			UPDATE
				resource
			SET
				deleted = NOW()
			WHERE
				id = $1;
			"#,
			runner.id as _,
		)
		.execute(&mut **database)
		.await?;

		query!(
			r#"
			UPDATE
				runner
			SET
				deleted = NOW()
			WHERE
				id = $1;
			"#,
			runner.id as _,
		)
		.execute(&mut **database)
		.await?;
	}

	AppResponse::builder()
		.body(DeleteRunnerResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
