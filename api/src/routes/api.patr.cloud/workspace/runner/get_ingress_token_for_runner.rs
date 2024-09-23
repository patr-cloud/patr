use axum::http::StatusCode;
use models::api::workspace::runner::*;
use rustis::commands::StringCommands;

use crate::prelude::*;

pub async fn get_ingress_token_for_runner(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetIngressTokenForRunnerPath {
					workspace_id,
					runner_id,
				},
				query: (),
				headers:
					GetIngressTokenForRunnerRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetIngressTokenForRunnerRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetIngressTokenForRunnerRequest>,
) -> Result<AppResponse<GetIngressTokenForRunnerRequest>, ErrorType> {
	info!("Getting ingress token for runner `{runner_id}`");

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

	// TODO get token from cloudflare

	AppResponse::builder()
		.body(GetIngressTokenForRunnerResponse { token: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
