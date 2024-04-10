use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::{
		workspace::infrastructure::deployment::*,
		WithId,
	},
	utils::StringifiedU16,
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn get_deployment_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentInfoPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentInfoRequest>,
) -> Result<AppResponse<GetDeploymentInfoRequest>, ErrorType> {
	info!("Starting: Get deployment info");

	let deployment_ports = query!(
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

	let deployment_env_variables = query!(
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

	let deployment_config_mounts = query!(
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

	let deployment_volumes = query!(
		r#"
		SELECT
			id,
			name,
			volume_size,
			volume_mount_path
		FROM
			deployment_volume
		WHERE
			deployment_id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|volume| {
		(
			volume.name,
			DeploymentVolume {
				path: volume.volume_mount_path,
				size: volume.volume_size as u16,
			},
		)
	})
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
			region,
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
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|deployment| GetDeploymentInfoResponse {
		deployment: WithId::new(
			deployment.id,
			Deployment {
				name: deployment.name,
				registry: if deployment.registry == PatrRegistry.to_string() {
					DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: deployment.repository_id.unwrap().into(),
					}
				} else {
					DeploymentRegistry::ExternalRegistry {
						registry: deployment.registry,
						image_name: deployment.image_name.unwrap().into(),
					}
				},
				image_tag: deployment.image_tag.into(),
				status: deployment.status,
				region: deployment.region.into(),
				machine_type: deployment.machine_type.into(),
				current_live_digest: deployment.current_live_digest,
			},
		),
		running_details: DeploymentRunningDetails {
			deploy_on_push: deployment.deploy_on_push,
			min_horizontal_scale: deployment.min_horizontal_scale as u16,
			max_horizontal_scale: deployment.max_horizontal_scale as u16,
			ports: deployment_ports,
			environment_variables: deployment_env_variables,
			startup_probe: Some(DeploymentProbe {
				port: deployment.startup_probe_port.unwrap() as u16,
				path: deployment.startup_probe_path.unwrap(),
			}),
			liveness_probe: Some(DeploymentProbe {
				port: deployment.liveness_probe_port.unwrap() as u16,
				path: deployment.liveness_probe_path.unwrap(),
			}),
			config_mounts: deployment_config_mounts,
			volumes: deployment_volumes,
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
