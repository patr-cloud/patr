use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// The handler to get the deployment info. This will return the deployment
/// details for the given deployment ID.
pub async fn get_deployment_info(
	AppRequest {
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
		runner_changes_sender: _,
		config: _,
	}: AppRequest<'_, GetDeploymentInfoRequest>,
) -> Result<AppResponse<GetDeploymentInfoRequest>, ErrorType> {
	info!("Getting deployment info");

	let ports = query(
		r#"
		SELECT
			port,
			port_type
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1
		"#,
	)
	.bind(deployment_id)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let port = row.try_get::<u16, _>("port")?;
		let port_type = row.try_get::<ExposedPortType, _>("port_type")?;

		Ok((StringifiedU16::new(port), port_type))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let environment_variables = query(
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
	)
	.bind(deployment_id)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|env| {
		let name = env.try_get::<String, _>("name")?;
		let value = env
			.try_get::<Option<String>, _>("value")?
			.map(EnvironmentVariableValue::String);

		let secret_id = env
			.try_get::<Option<Uuid>, _>("secret_id")?
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

	let config_mounts = query(
		r#"
		SELECT
			path,
			file
		FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let path = row.try_get::<String, _>("path")?;
		let file = row.try_get::<Vec<u8>, _>("file").map(Base64String::from)?;

		Ok((path, file))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let volumes = query(
		r#"
		SELECT
			volume_id,
			volume_mount_path
		FROM
			deployment_volume_mount
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let volume_id = row.try_get::<Uuid, _>("volume_id")?;
		let volume_mount_path = row.try_get::<String, _>("volume_mount_path")?;

		Ok((volume_id, volume_mount_path))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let deployment = query(
		r#"
		SELECT
			id,
			name,
			registry,
			image_name,
			image_tag,
			status,
			min_horizontal_scale,
			max_horizontal_scale,
			machine_type,
			deploy_on_push,
			startup_probe_port,
			startup_probe_path,
			startup_probe_port_type,
			liveness_probe_port,
			liveness_probe_path,
			liveness_probe_port_type,
			current_live_digest
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
	)
	.bind(deployment_id)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| {
		let deployment_id = row.try_get::<Uuid, _>("id")?;
		let name = row.try_get::<String, _>("name")?;
		let image_tag = row.try_get::<String, _>("image_tag")?;
		let status = row.try_get::<DeploymentStatus, _>("status")?;
		let registry = row.try_get::<String, _>("registry")?;
		let image_name = row.try_get::<String, _>("image_name")?;
		let machine_type = row.try_get::<Uuid, _>("machine_type")?;
		let current_live_digest = row.try_get::<Option<String>, _>("current_live_digest")?;

		let deploy_on_push = row.try_get::<bool, _>("deploy_on_push")?;
		let min_horizontal_scale = row.try_get::<u16, _>("min_horizontal_scale")?;
		let max_horizontal_scale = row.try_get::<u16, _>("max_horizontal_scale")?;

		Ok::<_, ErrorType>(GetDeploymentInfoResponse {
			deployment: WithId::new(
				deployment_id,
				Deployment {
					name,
					image_tag,
					status,
					registry: DeploymentRegistry::ExternalRegistry {
						registry,
						image_name,
					},
					// WARN: This is a dummy runner ID, as there is no runner-id in self-hosted PATR
					runner: Uuid::nil(),
					current_live_digest,
					machine_type,
				},
			),
			running_details: DeploymentRunningDetails {
				deploy_on_push,
				min_horizontal_scale,
				max_horizontal_scale,
				ports,
				environment_variables,
				startup_probe: row
					.try_get::<Option<u16>, _>("startup_probe_port")?
					.zip(row.try_get::<Option<String>, _>("startup_probe_path")?)
					.map(|(port, path)| DeploymentProbe { port, path }),
				liveness_probe: row
					.try_get::<Option<u16>, _>("liveness_probe_port")?
					.zip(row.try_get::<Option<String>, _>("liveness_probe_path")?)
					.map(|(port, path)| DeploymentProbe { port, path }),
				config_mounts,
				volumes,
			},
		})
	})
	.ok_or(ErrorType::ResourceDoesNotExist)??;

	AppResponse::builder()
		.body(deployment)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
