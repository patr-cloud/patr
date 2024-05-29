use axum::http::StatusCode;
use models::api::workspace::runner::*;

use crate::prelude::*;

pub async fn get_runner_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetRunnerInfoPath {
					workspace_id,
					runner_id,
				},
				query: (),
				headers:
					GetRunnerInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetRunnerInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetRunnerInfoRequest>,
) -> Result<AppResponse<GetRunnerInfoRequest>, ErrorType> {
	info!("Getting information about the workspace `{workspace_id}`");

	let runner = query!(
		r#"
		SELECT
			*
		FROM
			runner
		WHERE
			id = $1 AND
            workspace_id = $2 AND
			deleted IS NULL;
		"#,
		&runner_id as _,
		&workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetRunnerInfoResponse {
			runner: WithId::new(
				workspace_id,
				Runner {
					name: runner.name,
					connected: false, // TODO
					last_seen: None,  // TODO
				},
			),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
