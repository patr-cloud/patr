use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::{
		cfd_tunnel::{Tunnel, delete_tunnel},
		workerskv::delete_key,
	},
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
		response::ApiSuccess,
	},
};
use models::{api::workspace::runner::*, prelude::*};
use rustis::commands::GenericCommands as _;

use crate::{models::permissions, prelude::*};

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
		redis,
		client_ip: _,
		actor_data: _,
		state,
	}: AuthenticatedAppRequest<'_, DeleteRunnerRequest>,
) -> Result<AppResponse<DeleteRunnerRequest>, ErrorType> {
	info!("Deleting runner `{}`", runner_id);

	// Grab the tunnel id before the row is gone so we can tear it down on
	// Cloudflare afterwards. Locked so a concurrent ingress token fetch can't
	// create a tunnel this misses.
	let tunnel_id = query!(
		r#"
		SELECT
			cloudflare_tunnel_id
		FROM
			runner
		WHERE
			id = $1
		FOR UPDATE;
		"#,
		&runner_id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?
	.cloudflare_tunnel_id;

	// The `actor` and `actor_client` rows stay: `audit_log` points at them, so
	// removing them would break the trail of what this runner did.
	let Some(deleted) = query!(
		r#"
		DELETE FROM
			runner
		WHERE
			id = $1
		RETURNING
			token_hash;
		"#,
		runner_id,
	)
	.fetch_optional(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(dbe) if dbe.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => err.into(),
	})?
	else {
		return Err(ErrorType::ResourceDoesNotExist);
	};

	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		runner_id,
	)
	.execute(&mut **database)
	.await?;

	// Invalidate the cached workspace-for-runner lookup
	redis
		.del(redis::keys::workspace_id_for_runner(&runner_id))
		.await?;

	permissions::mark_token_stale(redis, &runner_id, &deleted.token_hash).await?;
	permissions::mark_actor_stale(redis, &runner_id).await?;

	// A runner still connected loses its lock and disconnects at its next ping
	redis
		.del(redis::keys::runner_connection_lock(&runner_id))
		.await?;

	cfg_if! {
		if #[cfg(feature = "cloud")] {
			let cloudflare = CloudflareClient::new(
				Credentials::UserAuthToken {
					token: state.config.cloudflare.api_key.clone(),
				},
				ClientConfig::default(),
				Environment::Custom(state.config.cloudflare.base_url.clone()),
			)?;

			cloudflare
				.request(&delete_key::DeleteKey {
					account_identifier: &state.config.cloudflare.account_id,
					namespace_identifier: &state.config.cloudflare.worker_namespace_id,
					key: &runner_id.to_string(),
				})
				.await?;

			// Delete the runner's Cloudflare tunnel too — otherwise it lingers on the
			// account forever. `cascade` tears down any active connections first. A
			// runner that never fetched an ingress token has no tunnel, and one
			// already gone from Cloudflare counts as deleted.
			if !tunnel_id.is_empty() {
				let response = reqwest::Client::new()
					.get(format!(
						"{}accounts/{}/cfd_tunnel/{}",
						state.config.cloudflare.base_url,
						state.config.cloudflare.account_id,
						tunnel_id
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
					cloudflare
						.request(&delete_tunnel::DeleteTunnel {
							account_identifier: &state.config.cloudflare.account_id,
							tunnel_id: &tunnel_id,
							params: delete_tunnel::Params { cascade: true },
						})
						.await?;
				}
			}
		} else {
			let _ = (state, tunnel_id);
		}
	}

	AppResponse::builder()
		.body(DeleteRunnerResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
