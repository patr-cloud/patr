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
use time::OffsetDateTime;

use crate::{
	prelude::*,
	redis_publisher::{self, DirtyResource},
};

/// The handler to start a deployment in the workspace. This will start
/// the deployment. In case the deployment is already running, it will
/// do nothing.
pub async fn start_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StartDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: StartDeploymentQueryProcessed { force_restart },
				headers:
					StartDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: StartDeploymentRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		actor_data: _,
		state,
	}: AuthenticatedAppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	info!("Starting deployment: {}", deployment_id);

	let now = OffsetDateTime::now_utc();

	let (registry, image_tag) = query!(
		r#"
		SELECT
			registry,
			repository_id,
			image_name,
			image_tag
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id
	)
	.fetch_optional(&mut **database)
	.await?
	.and_then(|deployment| {
		let registry = if deployment.registry == PatrRegistry.to_string() {
			DeploymentRegistry::PatrRegistry {
				registry: PatrRegistry,
				repository_id: deployment.repository_id?,
			}
		} else {
			DeploymentRegistry::ExternalRegistry {
				registry: deployment.registry,
				image_name: deployment.image_name?,
			}
		};
		Some((registry, deployment.image_tag))
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	let digest = if let DeploymentRegistry::PatrRegistry { repository_id, .. } = &registry {
		let digest = query!(
			r#"
			SELECT
				manifest_digest
			FROM
				container_registry_repository_tag
			WHERE
				repository_id = $1 AND
				name = $2;
			"#,
			repository_id,
			image_tag
		)
		.fetch_optional(&mut **database)
		.await?
		.map(|row| row.manifest_digest);

		if let Some(digest) = &digest {
			// Check if digest is already in deployment_deploy_history table
			// If not, add it to the table
			query!(
				r#"
				INSERT INTO
					deployment_deploy_history(
						deployment_id,
						image_digest,
						repository_id,
						created
					)
				VALUES
					($1, $2, $3, $4)
				ON CONFLICT
					(deployment_id, image_digest)
				DO NOTHING;
				"#,
				deployment_id,
				digest,
				repository_id,
				now,
			)
			.execute(&mut **database)
			.await?;
		}

		digest
	} else {
		None
	};

	// Set the deployment to run and set the current live digest
	let deployment = query!(
		r#"
		UPDATE
			deployment
		SET
			should_run = TRUE,
			current_live_digest = COALESCE($2, current_live_digest)
		WHERE
			id = $1
		RETURNING
			runner,
			should_run;
		"#,
		deployment_id,
		digest,
	)
	.fetch_one(&mut **database)
	.await?;

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
		.body(StartDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
