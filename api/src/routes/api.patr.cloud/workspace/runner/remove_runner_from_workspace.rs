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

	// Grab the tunnel id and the service account before the row is gone: the
	// tunnel gets torn down on Cloudflare afterwards, and the service account
	// cascade below needs to know which SA this runner owned.
	let runner = query!(
		r#"
		SELECT
			cloudflare_tunnel_id,
			service_account_id AS "service_account_id: Uuid"
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

	let tunnel_id = runner.cloudflare_tunnel_id;
	let sa_id = runner.service_account_id;

	// Every grant the runner's service account held. There is no per-runner
	// role to clean up any more — the two roles it was bound to are immutable
	// workspace defaults, shared by every runner.
	query!(
		r#"
		DELETE FROM
			role_binding
		WHERE
			actor_id = $1;
		"#,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Drop runner first to release its FK to service_account.
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
		DELETE FROM
			service_account
		WHERE
			id = $1;
		"#,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	// The actor row goes with the account; its bindings are already gone. The
	// `actor_client` row deliberately stays — `audit_log` points at it, so
	// removing it would break the trail of what this runner did.
	query!(
		r#"
		DELETE FROM
			workspace_actor
		WHERE
			id = $1;
		"#,
		sa_id as _,
	)
	.execute(&mut **database)
	.await?;

	// Soft-delete the underlying resource rows so audit logs / FKs to
	// `resource` remain consistent (matches the existing pattern elsewhere).
	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id IN ($1, $2);
		"#,
		runner_id as _,
		sa_id as _,
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
			cloudflare
				.request(&delete_tunnel::DeleteTunnel {
					account_identifier: &state.config.cloudflare.account_id,
					tunnel_id: &tunnel_id,
					params: delete_tunnel::Params { cascade: true },
				})
				.await?;
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
