use std::collections::BTreeMap;

use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::prelude::*;

pub async fn get_deployment_info(
	request: AppRequest<'_, GetDeploymentInfoRequest>,
) -> Result<AppResponse<GetDeploymentInfoRequest>, ErrorType> {
	let AppRequest {
		database,
		request:
			ProcessedApiRequest {
				path: GetDeploymentInfoPath {
					workspace_id: _,
					deployment_id,
				},
				query: _,
				headers: _,
				body: _,
			},
	} = request;
	info!("Getting deployment info");

	let ports = query(
		r#"
		SELECT
			port,
			port_type,
		FROM
			deployment_exposed_ports
		WHERE
			deployment_id = $1

	"#,
	)
	.bind(deployment_id)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| {
		let port = row.try_get::<i32, &str>("port")?;
		let port_type = row
			.try_get::<String, &str>("port_type")?
			.parse::<ExposedPortType>()?;

		Ok((StringifiedU16::new(port as u16), port_type))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let environment_variables = query(
		r#"
		SELECT
			name,
			value,
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
		let name = env.try_get::<String, &str>("name")?;
		let value = env
			.try_get::<String, &str>("value")
			.map(|val| EnvironmentVariableValue::String(val))?;

		Ok((name, value))
	})
	.collect::<Result<BTreeMap<String, EnvironmentVariableValue>, ErrorType>>()?;

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
		let path = row.try_get::<String, &str>("path")?;
		let file = row
			.try_get::<Vec<u8>, &str>("file")
			.map(|file| Base64String::from(file))?;

		Ok((path, file))
	})
	.collect::<Result<BTreeMap<String, Base64String>, ErrorType>>()?;

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
		let volume_id = row.try_get::<Uuid, &str>("volume_id")?;
		let volume_mount_path = row.try_get::<String, &str>("volume_mount_path")?;

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
	.map(|row| -> Result<GetDeploymentInfoResponse, ErrorType> {
		let name = row.try_get::<String, &str>("name").map_err(|err| {
			ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
		})?;
		let repository = row.try_get::<String, &str>("registry").map_err(|err| {
			ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
		})?;
		let image_name = row.try_get::<String, &str>("image_name").map_err(|err| {
			ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
		})?;
		let image_tag = row.try_get::<String, &str>("image_tag").map_err(|err| {
			ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
		})?;

		let registry = DeploymentRegistry::ExternalRegistry {
			registry: repository,
			image_name,
		};

		let status = row
			.try_get::<String, &str>("status")?
			.parse::<DeploymentStatus>()
			.map_err(|err| {
				ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
			})?;

		let machine_type = row
			.try_get::<String, &str>("machine_type")?
			.parse::<Uuid>()
			.map_err(|err| {
				ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
			})?;

		let current_live_digest = row
			.try_get::<Option<String>, &str>("current_live_digest")
			.map_err(|err| {
				ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
			})?;

		let deploy_on_push = row.try_get::<bool, &str>("deploy_on_push").map_err(|err| {
			ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
		})?;

		let min_horizontal_scale =
			row.try_get::<u16, &str>("min_horizontal_scale")
				.map_err(|err| {
					ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
				})?;

		let max_horizontal_scale =
			row.try_get::<u16, &str>("max_horizontal_scale")
				.map_err(|err| {
					ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
				})?;

		let startup_port_port = row
			.try_get::<Option<u16>, &str>("startup_probe_port")
			.map_err(|_| {
				ErrorType::server_error("corrupted deployment, cannot find startup_probe_port")
			})?;

		let startup_port_path = row
			.try_get::<Option<String>, &str>("startup_probe_path")
			.map_err(|_| {
				ErrorType::server_error("corrupted deployment, cannot find startup_probe_path")
			})?;

		let startup_probe = startup_port_port
			.zip(startup_port_path)
			.map(|(port, path)| DeploymentProbe { port, path });

		let liveness_port_port = row
			.try_get::<Option<u16>, &str>("liveness_probe_port")
			.map_err(|err| {
				ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
			})?;

		let liveness_port_path = row
			.try_get::<Option<String>, &str>("liveness_probe_path")
			.map_err(|err| {
				ErrorType::server_error(format!("corrupted deployment, {}", err.to_string()))
			})?;

		let liveness_probe = liveness_port_port
			.zip(liveness_port_path)
			.map(|(port, path)| DeploymentProbe { port, path });

		Ok(GetDeploymentInfoResponse {
			deployment: WithId::new(
				deployment_id,
				Deployment {
					name,
					registry,
					image_tag,
					status,
					machine_type,
					runner: Uuid::nil(),
					current_live_digest,
				},
			),
			running_details: DeploymentRunningDetails {
				deploy_on_push,
				min_horizontal_scale,
				max_horizontal_scale,
				ports,
				startup_probe,
				liveness_probe,
				environment_variables,
				volumes,
				config_mounts,
			},
		})
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?
	.map_err(|err| ErrorType::server_error(format!("corrupted deployment, {}", err.to_string())))?;

	AppResponse::builder()
		.body(deployment)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
