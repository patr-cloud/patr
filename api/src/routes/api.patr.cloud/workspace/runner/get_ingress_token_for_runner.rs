#[cfg(feature = "cloud")]
use cloudflare::{endpoints::cfd_tunnel::Tunnel, framework::response::ApiSuccess};
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

	cfg_if! {
		if #[cfg(feature = "cloud")] {
			use axum::http::StatusCode;

			let runner = query!(
				r#"
				SELECT
					cloudflare_tunnel_id
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

			let client = reqwest::Client::new();

			// Check if the tunnel still exists on Cloudflare
			let tunnel_exists = client
				.get(format!(
					"{}accounts/{}/cfd_tunnel/{}",
					state.config.cloudflare.base_url,
					state.config.cloudflare.account_id,
					runner.cloudflare_tunnel_id
				))
				.bearer_auth(&state.config.cloudflare.api_key)
				.send()
				.await?
				.json::<ApiSuccess<Option<Tunnel>>>()
				.await?
				.result
				.filter(|tunnel| tunnel.deleted_at.is_none())
				.is_some();

			// If the tunnel was deleted or removed, recreate it with catch-all config
			let tunnel_id = if tunnel_exists {
				runner.cloudflare_tunnel_id
			} else {
				warn!("Tunnel for runner `{runner_id}` not found on Cloudflare, recreating");

				let new_tunnel_id =
					utils::cloudflare::create_tunnel_with_config(runner_id, &state.config).await?;

				query!(
					r#"
					UPDATE
						runner
					SET
						cloudflare_tunnel_id = $1
					WHERE
						id = $2;
					"#,
					&new_tunnel_id,
					runner_id as _,
				)
				.execute(&mut **database)
				.await?;

				new_tunnel_id
			};

			trace!("Getting the tunnel token for the runner");
			let token = client
				.get(format!(
					"{}accounts/{}/cfd_tunnel/{}/token",
					state.config.cloudflare.base_url, state.config.cloudflare.account_id, tunnel_id
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
		} else {
			let _ = (runner_id, database, state);
			Err(ErrorType::FeatureNotSupported)
		}
	}
}
