use axum::http::StatusCode;
use models::api::workspace::runner::*;
use semver::Version;

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
		actor_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, GetRunnerInfoRequest>,
) -> Result<AppResponse<GetRunnerInfoRequest>, ErrorType> {
	info!("Getting information about the runner `{runner_id}`");

	let runner = query!(
		r#"
		SELECT
			name,
			IS_RUNNER_ONLINE(is_connected, last_seen) AS "connected!",
			last_seen,
			version
		FROM
			runner
		WHERE
			id = $1 AND
			workspace_id = $2 AND
			deleted IS NULL;
		"#,
		&runner_id,
		&workspace_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetRunnerInfoResponse {
			runner: WithId::new(
				runner_id,
				Runner {
					name: runner.name,
					connected: runner.connected,
					last_seen: runner.last_seen,
					version: runner
						.version
						.parse::<Version>()
						.unwrap_or_else(|_| Version::new(0, 0, 0)),
				},
			),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
