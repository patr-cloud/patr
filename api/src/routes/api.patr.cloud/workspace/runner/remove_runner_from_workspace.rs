use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::{cfd_tunnel::delete_tunnel, workerskv::delete_key},
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::{api::workspace::runner::*, prelude::*};
use rustis::commands::GenericCommands as _;

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
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, DeleteRunnerRequest>,
) -> Result<AppResponse<DeleteRunnerRequest>, ErrorType> {
	info!("Deleting runner `{}`", runner_id);

	// Grab the tunnel id before the row is gone so we can tear it down on
	// Cloudflare afterwards.
	let tunnel_id = query!(
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
	.map(|runner| runner.cloudflare_tunnel_id);

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

	// Invalidate the cached workspace-for-runner lookup
	redis
		.del(redis::keys::workspace_id_for_runner(&runner_id))
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
			// account forever. `cascade` tears down any active connections first.
			if let Some(tunnel_id) = tunnel_id {
				cloudflare
					.request(&delete_tunnel::DeleteTunnel {
						account_identifier: &state.config.cloudflare.account_id,
						tunnel_id: &tunnel_id,
						params: delete_tunnel::Params { cascade: true },
					})
					.await?;
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
