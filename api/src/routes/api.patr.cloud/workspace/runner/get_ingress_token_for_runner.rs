#[cfg(feature = "cloud")]
use cloudflare::{endpoints::cfd_tunnel::Tunnel, framework::response::ApiSuccess};
use models::api::workspace::runner::*;

#[cfg(feature = "cloud")]
use crate::models::permissions;
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
						authorization,
						user_agent: _,
					},
				body: GetIngressTokenForRunnerRequestProcessed,
			},
		database,
		redis,
		client_ip,
		actor_data: _,
		state,
	}: AuthenticatedAppRequest<'_, GetIngressTokenForRunnerRequest>,
) -> Result<AppResponse<GetIngressTokenForRunnerRequest>, ErrorType> {
	info!("Getting ingress token for runner `{runner_id}`");

	cfg_if! {
		if #[cfg(feature = "cloud")] {
			use axum::http::StatusCode;

			// Locked so a regenerate or delete in flight finishes first, and two
			// fetches can't both create a tunnel
			let runner = query!(
				r#"
				SELECT
					cloudflare_tunnel_id
				FROM
					runner
				WHERE
					id = $1
				FOR UPDATE;
				"#,
				&runner_id as _,
			)
			.fetch_optional(&mut **database)
			.await?
			.ok_or(ErrorType::ResourceDoesNotExist)?;

			// Authenticate again now that the runner is locked. Say Mallory has
			// a leaked copy of Alice's runner token and keeps calling this, and
			// Alice regenerates the token. One of Mallory's calls gets past the
			// authenticator middleware on a cached entry just before the
			// regenerate clears it, then waits on the lock above while the
			// regenerate swaps the tunnel. Once the regenerate commits, that
			// call would hand Mallory the new tunnel's token. Authenticating
			// again misses the cache and finds the old token gone.
			permissions::authenticate(
				database,
				redis,
				&state.config,
				client_ip,
				authorization.0.token(),
				<GetIngressTokenForRunnerRequest as ApiEndpoint>::ALLOWED_CLIENT_TYPES,
			)
			.await?;

			let client = reqwest::Client::new();

			// Check if the tunnel still exists on Cloudflare. A runner has no tunnel
			// until its first fetch here, and `GET cfd_tunnel/` with an empty id
			// hits the list endpoint, so that case skips the lookup.
			let tunnel_exists = !runner.cloudflare_tunnel_id.is_empty() &&
				client
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

			// If the tunnel was never created, or was deleted or removed, create it
			// with catch-all config
			let tunnel_id = if tunnel_exists {
				runner.cloudflare_tunnel_id
			} else {
				if runner.cloudflare_tunnel_id.is_empty() {
					info!("Creating the tunnel for runner `{runner_id}`");
				} else {
					warn!("Tunnel for runner `{runner_id}` not found on Cloudflare, recreating");
				}

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
			let _ = (runner_id, authorization, database, redis, client_ip, state);
			Err(ErrorType::FeatureNotSupported)
		}
	}
}
