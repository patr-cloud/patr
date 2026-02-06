use axum::http::StatusCode;
use cloudflare::framework::response::ApiSuccess;
use models::api::workspace::runner::*;

use crate::prelude::*;

pub async fn get_ingress_token_for_runner(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetIngressTokenForRunnerPath {
					workspace_id: _,
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
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, GetIngressTokenForRunnerRequest>,
) -> Result<AppResponse<GetIngressTokenForRunnerRequest>, ErrorType> {
	info!("Getting ingress token for runner `{runner_id}`");

	super::update_cloudflare_config_for_runner(runner_id, &mut **database, &state.config).await?;

	let runner = query!(
		r#"
		SELECT
			*
		FROM
			runner
		WHERE
			id = $1;
		"#,
		&runner_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	trace!("Getting the tunnel token for the runner");
	let token = reqwest::Client::new()
		.get(format!(
			"https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/token",
			state.config.cloudflare.account_id, runner.cloudflare_tunnel_id
		))
		.bearer_auth(&state.config.cloudflare.api_key)
		.send()
		.await?
		.json::<ApiSuccess<String>>()
		.await?
		.result;

	AppResponse::builder()
		.body(GetIngressTokenForRunnerResponse { token })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
