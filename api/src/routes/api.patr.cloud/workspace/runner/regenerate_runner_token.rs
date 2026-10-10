use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::cfd_tunnel::{Tunnel, delete_tunnel},
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
		response::ApiSuccess,
	},
};
use models::{api::workspace::runner::*, prelude::*};
use rustis::commands::GenericCommands as _;
use sha2::{Digest as _, Sha256};

use crate::{models::permissions, prelude::*};

pub async fn regenerate_runner_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: RegenerateRunnerTokenPath {
					workspace_id,
					runner_id,
				},
				query: (),
				headers:
					RegenerateRunnerTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RegenerateRunnerTokenRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		actor_data: _,
		state,
	}: AuthenticatedAppRequest<'_, RegenerateRunnerTokenRequest>,
) -> Result<AppResponse<RegenerateRunnerTokenRequest>, ErrorType> {
	info!("Regenerating the token for runner `{runner_id}`");

	// The authorizer only proves the id is some resource in the workspace, so
	// confirm it's a runner, and take the service account to rotate from it.
	// Locked so deleting the runner or fetching its ingress token waits for the
	// rotation.
	let runner = query!(
		r#"
		SELECT
			service_account_id AS "service_account_id: Uuid",
			cloudflare_tunnel_id
		FROM
			runner
		WHERE
			id = $1 AND
			workspace_id = $2 AND
			deleted IS NULL
		FOR UPDATE;
		"#,
		runner_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let sa_id = runner.service_account_id;

	let token = permissions::generate_service_account_token();
	let token_hash = hex::encode(Sha256::digest(&token));

	// `old` is the row as it was before the update (Postgres 18), so this hands
	// back the hash being replaced, whose cache entry has to go.
	let Some(old) = query!(
		r#"
		UPDATE
			service_account
		SET
			token_hash = $1
		WHERE
			id = $2 AND
			deleted IS NULL
		RETURNING
			old.token_hash;
		"#,
		&token_hash,
		sa_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::ResourceDoesNotExist);
	};

	// Swap the tunnel too, so a connector still running on the old machine
	// loses it. A runner with no tunnel yet gets one on its first ingress token
	// fetch.
	cfg_if! {
		if #[cfg(feature = "cloud")] {
			if !runner.cloudflare_tunnel_id.is_empty() {
				// A tunnel already gone from Cloudflare counts as deleted
				let response = reqwest::Client::new()
					.get(format!(
						"{}accounts/{}/cfd_tunnel/{}",
						state.config.cloudflare.base_url,
						state.config.cloudflare.account_id,
						runner.cloudflare_tunnel_id
					))
					.bearer_auth(&state.config.cloudflare.api_key)
					.send()
					.await?;
				let tunnel_exists = response.status() != StatusCode::NOT_FOUND &&
					response
						.error_for_status()?
						.json::<ApiSuccess<Option<Tunnel>>>()
						.await?
						.result
						.is_some_and(|tunnel| tunnel.deleted_at.is_none());

				if tunnel_exists {
					CloudflareClient::new(
						Credentials::UserAuthToken {
							token: state.config.cloudflare.api_key.clone(),
						},
						ClientConfig::default(),
						Environment::Custom(state.config.cloudflare.base_url.clone()),
					)?
					.request(&delete_tunnel::DeleteTunnel {
						account_identifier: &state.config.cloudflare.account_id,
						tunnel_id: &runner.cloudflare_tunnel_id,
						params: delete_tunnel::Params { cascade: true },
					})
					.await?;
				}

				let tunnel_id =
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
					&tunnel_id,
					runner_id as _,
				)
				.execute(&mut **database)
				.await?;
			}
		} else {
			let _ = (state, runner.cloudflare_tunnel_id);
		}
	}

	permissions::mark_token_stale(redis, &sa_id, &old.token_hash).await?;
	permissions::mark_actor_stale(redis, &sa_id).await?;

	// A runner connected with the old token loses its lock and disconnects at
	// its next ping
	redis
		.del(redis::keys::runner_connection_lock(&runner_id))
		.await?;

	AppResponse::builder()
		.body(RegenerateRunnerTokenResponse { token })
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
