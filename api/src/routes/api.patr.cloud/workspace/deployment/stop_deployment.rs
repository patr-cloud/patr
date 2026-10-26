use std::collections::BTreeMap;

use axum::http::StatusCode;
#[cfg(feature = "cloud")]
use cloudflare::{
	endpoints::workerskv::write_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
#[cfg(feature = "cloud")]
use models::cloudflare::kv::*;
use models::{api::workspace::deployment::*, rbac::ResourceType, utils::StringifiedU16};

use crate::{
	prelude::*,
	redis_publisher::{self, DirtyResource},
};

/// The handler to stop a deployment in the workspace. This will stop
/// the deployment. In case the deployment is already stopped, it will
/// do nothing.
pub async fn stop_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StopDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers:
					StopDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: StopDeploymentRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		actor_data: _,
		state,
	}: AuthenticatedAppRequest<'_, StopDeploymentRequest>,
) -> Result<AppResponse<StopDeploymentRequest>, ErrorType> {
	info!("Starting: Stop deployment");

	// Set the deployment to stop
	let deployment = query!(
		r#"
		UPDATE
			deployment
		SET
			should_run = FALSE
		WHERE
			id = $1 AND
			deleted IS NULL
		RETURNING
			runner,
			should_run;
		"#,
		deployment_id
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let ports = query!(
		r#"
		SELECT
			port,
			port_type
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let port = row.port as u16;
		let port_type = row.port_type;

		Ok((StringifiedU16::new(port), port_type))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	cfg_if! {
		if #[cfg(feature = "cloud")] {
			CloudflareClient::new(
				Credentials::UserAuthToken {
					token: state.config.cloudflare.api_key.clone(),
				},
				ClientConfig::default(),
				Environment::Custom(state.config.cloudflare.base_url.clone()),
			)?
			.request(&write_key::WriteKey {
				account_identifier: &state.config.cloudflare.account_id,
				namespace_identifier: &state.config.cloudflare.worker_namespace_id,
				key: &deployment_id.to_string(),
				params: write_key::WriteKeyParams {
					expiration: None,
					expiration_ttl: None,
				},
				body: write_key::WriteKeyBody::Value(serde_json::to_vec(
					&InternalKVData::Deployment {
						ports: ports.keys().map(|port| port.value()).collect(),
						runner_id: deployment.runner,
						should_run: deployment.should_run,
					},
				)?),
			})
			.await?;
		} else {
			let _ = (state, ports); // Avoid unused variable warning
		}
	}

	redis_publisher::mark_dirty(
		database,
		DirtyResource {
			workspace_id,
			runner_id: deployment.runner,
			resource_type: ResourceType::Deployment,
			resource_id: deployment_id,
		},
	)
	.await?;

	AppResponse::builder()
		.body(StopDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
