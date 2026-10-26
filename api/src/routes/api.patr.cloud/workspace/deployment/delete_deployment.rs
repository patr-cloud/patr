use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::workerskv::delete_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::{api::workspace::deployment::*, rbac::ResourceType};
use rustis::commands::GenericCommands as _;

use crate::{
	prelude::*,
	redis_publisher::{self, DirtyResource},
};

/// The handler to delete a deployment in the workspace. This will delete the
/// deployment from the workspace, and remove all resources associated with the
/// deployment.
pub async fn delete_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: (),
				headers:
					DeleteDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteDeploymentRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		actor_data: _,
		state,
	}: AuthenticatedAppRequest<'_, DeleteDeploymentRequest>,
) -> Result<AppResponse<DeleteDeploymentRequest>, ErrorType> {
	info!("Deleting deployment: {deployment_id}");

	let runner = query!(
		r#"
		SELECT
			runner
		FROM
			deployment
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?
	.runner;

	query!(
		r#"
		DELETE FROM
			deployment_volume
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut **database)
	.await?;

	// Break the FK cycle between deployment.current_live_digest and
	// deployment_deploy_history before deleting history rows — both FKs
	// are non-deferrable, so SET CONSTRAINTS ALL DEFERRED below is a no-op
	// for them.
	query!(
		r#"
		UPDATE
			deployment
		SET
			current_live_digest = NULL
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut **database)
	.await?;

	// Delete the deployment in the database
	query!(
		r#"
		DELETE FROM
			deployment
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => ErrorType::server_error(err),
	})?;

	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut **database)
	.await?;

	// Mark the resource as deleted in the database
	query!(
		r#"
		UPDATE
			resource
		SET
			deleted = NOW()
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.execute(&mut **database)
	.await?;

	// Invalidate the cached runner-for-deployment lookup
	redis
		.del(redis::keys::runner_id_for_deployment(&deployment_id))
		.await?;

	cfg_if! {
		if #[cfg(feature = "cloud")] {
			CloudflareClient::new(
				Credentials::UserAuthToken {
					token: state.config.cloudflare.api_key.clone(),
				},
				ClientConfig::default(),
				Environment::Custom(state.config.cloudflare.base_url.clone()),
			)?
			.request(&delete_key::DeleteKey {
				account_identifier: &state.config.cloudflare.account_id,
				namespace_identifier: &state.config.cloudflare.worker_namespace_id,
				key: &deployment_id.to_string(),
			})
			.await?;
		} else {
			let _ = state; // Avoid unused variable warning
		}
	}

	redis_publisher::mark_dirty(
		database,
		DirtyResource {
			workspace_id,
			runner_id: runner,
			resource_type: ResourceType::Deployment,
			resource_id: deployment_id,
		},
	)
	.await?;

	AppResponse::builder()
		.body(DeleteDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
