use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::{
	api::workspace::{deployment::*, runner::StreamRunnerDataForWorkspaceServerMsg},
	utils::{Base64String, StringifiedU16},
};
use rustis::commands::PubSubCommands;
use time::OffsetDateTime;

use crate::prelude::*;

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
				query: StartDeploymentQuery { force_restart },
				headers:
					StartDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: StartDeploymentRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	info!("Starting deployment: {}", deployment_id);

	let now = OffsetDateTime::now_utc();

	let (registry, image_tag, region) = query!(
		r#"
		SELECT
			registry,
			repository_id,
			image_name,
			image_tag,
			runner
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.and_then(|deployment| {
		let registry = if deployment.registry == PatrRegistry.to_string() {
			DeploymentRegistry::PatrRegistry {
				registry: PatrRegistry,
				repository_id: deployment.repository_id?.into(),
			}
		} else {
			DeploymentRegistry::ExternalRegistry {
				registry: deployment.registry,
				image_name: deployment.image_name?,
			}
		};
		Some((registry, deployment.image_tag, deployment.runner))
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	if let DeploymentRegistry::PatrRegistry { repository_id, .. } = &registry {
		let digest = query!(
			r#"
			SELECT
				manifest_digest
			FROM
				container_registry_repository_manifest
			WHERE
				repository_id = $1
			ORDER BY
				created_at DESC
			LIMIT 1;
			"#,
			repository_id as _
		)
		.fetch_optional(&mut **database)
		.await?
		.map(|row| row.manifest_digest);

		if let Some(digest) = digest {
			// Check if digest is already in deployment_deploy_history table
			let deployment_deploy_history = query!(
				r#"
				SELECT
					image_digest,
					created
				FROM
					deployment_deploy_history
				WHERE
					image_digest = $1;
				"#,
				digest as _,
			)
			.fetch_optional(&mut **database)
			.await?;

			// If not, add it to the table
			if deployment_deploy_history.is_none() {
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
					deployment_id as _,
					digest as _,
					repository_id as _,
					now as _,
				)
				.execute(&mut **database)
				.await?;
			}
		}
	}

	// Update status to deploying
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		DeploymentStatus::Deploying as _,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	// TODO Temporary workaround until audit logs and triggers are implemented
	let ports = query!(
		r#"
		SELECT
			port,
			port_type AS "port_type: ExposedPortType"
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
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

	let environment_variables = query!(
		r#"
		SELECT
			name,
			value,
			secret_id AS "secret_id: Uuid"
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|env| {
		let name = env.name;
		let value = env.value.map(EnvironmentVariableValue::String);

		let secret_id = env
			.secret_id
			.map(|from_secret| EnvironmentVariableValue::Secret { from_secret });

		let value = match (value, secret_id) {
			(Some(value), None) => Some(value),
			(None, Some(secret)) => Some(secret),
			_ => None,
		}
		.ok_or(ErrorType::server_error(
			"corrupted deployment, cannot find environment variable value",
		))?;

		Ok((name, value))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let config_mounts = query!(
		r#"
		SELECT
			path,
			file
		FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let path = row.path;
		let file = Base64String::from(row.file);

		Ok((path, file))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let volumes = query!(
		r#"
		SELECT
			volume_id AS "volume_id: Uuid",
			volume_mount_path
		FROM
			deployment_volume_mount
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let volume_id = row.volume_id;
		let volume_mount_path = row.volume_mount_path;

		Ok((volume_id, volume_mount_path))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let row = query!(
		r#"
		SELECT
			id AS "id: Uuid",
			name,
			registry,
			image_name,
			image_tag,
			runner AS "runner: Uuid",
			status AS "status: DeploymentStatus",
			repository_id AS "repository_id: Uuid",
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type AS "machine_type: Uuid",
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			startup_probe_port_type AS "startup_probe_port_type: Option<ExposedPortType>",
			liveness_probe_port,
			liveness_probe_path,
			liveness_probe_port_type AS "liveness_probe_port_type: Option<ExposedPortType>",
			current_live_digest
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::RowNotFound => ErrorType::ResourceDoesNotExist,
		err => err.into(),
	})?;

	let name = row.name;
	let image_tag = row.image_tag;
	let machine_type = row.machine_type;
	let runner = row.runner;

	let deploy_on_push = row.deploy_on_push;
	let min_horizontal_scale = row.min_horizontal_scale as u16;
	let max_horizontal_scale = row.max_horizontal_scale as u16;

	let startup_probe = row
		.startup_probe_port
		.zip(row.startup_probe_path)
		.map(|(port, path)| DeploymentProbe {
			port: port as u16,
			path,
		});

	let liveness_probe =
		row.liveness_probe_port
			.zip(row.liveness_probe_path)
			.map(|(port, path)| DeploymentProbe {
				port: port as u16,
				path,
			});

	redis
		.publish(
			format!("{}/runner/{}/stream", workspace_id, runner),
			serde_json::to_string(&StreamRunnerDataForWorkspaceServerMsg::DeploymentUpdated {
				deployment: WithId::new(
					deployment_id,
					Deployment {
						name: name.to_string(),
						registry: if row.registry == PatrRegistry.to_string() {
							DeploymentRegistry::PatrRegistry {
								registry: PatrRegistry,
								repository_id: row.repository_id.unwrap(),
							}
						} else {
							DeploymentRegistry::ExternalRegistry {
								registry: row.registry,
								image_name: row.image_name.unwrap(),
							}
						},
						image_tag: image_tag.to_string(),
						runner,
						status: DeploymentStatus::Deploying,
						current_live_digest: row.current_live_digest,
						machine_type,
					},
				),
				running_details: DeploymentRunningDetails {
					deploy_on_push,
					min_horizontal_scale,
					max_horizontal_scale,
					ports,
					environment_variables,
					startup_probe,
					liveness_probe,
					config_mounts,
					volumes,
				},
			})
			.unwrap(),
		)
		.await?;

	AppResponse::builder()
		.body(StartDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
