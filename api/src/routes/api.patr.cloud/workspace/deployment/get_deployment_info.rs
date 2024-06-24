use axum::http::StatusCode;
use models::{api::workspace::deployment::*, utils::StringifiedU16};

use crate::prelude::*;

/// The handler to get the deployment info in the workspace. This will return
/// the deployment details for the given deployment ID.
pub async fn get_deployment_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentInfoPath {
					workspace_id: _,
					deployment_id,
				},
				query: (),
				headers:
					GetDeploymentInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetDeploymentInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetDeploymentInfoRequest>,
) -> Result<AppResponse<GetDeploymentInfoRequest>, ErrorType> {
	info!("Getting deployment info");

	let ports = query!(
		r#"
		SELECT
			port,
			port_type as "port_type: ExposedPortType"
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
	.map(|row| (StringifiedU16::new(row.port as u16), row.port_type))
	.collect();

	let environment_variables = query!(
		r#"
		SELECT
			name,
			value,
			secret_id
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
	.filter_map(|env| match (env.value, env.secret_id) {
		(Some(value), None) => Some((env.name, EnvironmentVariableValue::String(value))),
		(None, Some(secret_id)) => Some((
			env.name,
			EnvironmentVariableValue::Secret {
				from_secret: secret_id.into(),
			},
		)),
		_ => None,
	})
	.collect();

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
	.map(|mount| (mount.path, mount.file.into()))
	.collect();

	let volumes = query!(
		r#"
		SELECT
			volume_id,
			volume_mount_path
		FROM
			deployment_volume_mount
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| (row.volume_id.into(), row.volume_mount_path))
	.collect();

	let deployment = query!(
		r#"
		SELECT
			id,
			name,
			registry,
			repository_id,
			image_name,
			image_tag,
			status as "status: DeploymentStatus",
			workspace_id,
			runner,
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type,
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			liveness_probe_port,
			liveness_probe_path,
			current_live_digest
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
	.map(|row| GetDeploymentInfoResponse {
		deployment: WithId::new(
			row.id,
			Deployment {
				name: row.name,
				registry: if row.registry == PatrRegistry.to_string() {
					DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: row.repository_id.unwrap().into(),
					}
				} else {
					DeploymentRegistry::ExternalRegistry {
						registry: row.registry,
						image_name: row.image_name.unwrap(),
					}
				},
				image_tag: row.image_tag,
				status: row.status,
				runner: row.runner.into(),
				machine_type: row.machine_type.into(),
				current_live_digest: row.current_live_digest,
			},
		),
		running_details: DeploymentRunningDetails {
			deploy_on_push: row.deploy_on_push,
			min_horizontal_scale: row.min_horizontal_scale as u16,
			max_horizontal_scale: row.max_horizontal_scale as u16,
			ports,
			environment_variables,
			startup_probe: row.startup_probe_port.zip(row.startup_probe_path).map(
				|(port, path)| DeploymentProbe {
					port: port as u16,
					path,
				},
			),
			liveness_probe: row.liveness_probe_port.zip(row.liveness_probe_path).map(
				|(port, path)| DeploymentProbe {
					port: port as u16,
					path,
				},
			),
			config_mounts,
			volumes,
		},
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(deployment)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
