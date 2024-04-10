use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::workspace::infrastructure::deployment::*,
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::{models::deployment::MACHINE_TYPES, prelude::*};

pub async fn update_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateDeploymentPath {
					workspace_id,
					deployment_id,
				},
				query: _,
				headers,
				body:
					UpdateDeploymentRequestProcessed {
						name,
						machine_type,
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
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, UpdateDeploymentRequest>,
) -> Result<AppResponse<UpdateDeploymentRequest>, ErrorType> {
	info!("Starting: List linked URLs");

	// Validate if at least value is to be updated
	if name.is_none() &&
		machine_type.is_none() &&
		deploy_on_push.is_none() &&
		min_horizontal_scale.is_none() &&
		max_horizontal_scale.is_none() &&
		ports.is_none() &&
		environment_variables.is_none() &&
		startup_probe.is_none() &&
		liveness_probe.is_none() &&
		config_mounts.is_none() &&
		volumes.is_none()
	{
		return Err(ErrorType::WrongParameters);
	}

	let deployment = query!(
		r#"
		SELECT
			region,
			min_horizontal_scale
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	// Get volume size to check limit
	let volume_size = if let Some(volume) = volumes {
		volume
			.iter()
			.map(|(_, volume)| volume.size as u32)
			.sum::<u32>()
	} else {
		0
	};

	if todo!("Check if deployment in patr region and card not added") {
		if let Some(machine_type) = machine_type {
			// only basic machine type is allowed under free plan
			let machine_type_to_be_deployed = MACHINE_TYPES
				.get()
				.and_then(|machines| machines.get(&machine_type))
				.ok_or(ErrorType::server_error("Failed to get machine type info"))?;

			if machine_type_to_be_deployed != &(1, 2) {
				return Err(ErrorType::FreeLimitExceeded);
			}
		}
		if let Some(max_horizontal_scale) = max_horizontal_scale {
			if max_horizontal_scale > 1 {
				return Err(ErrorType::FreeLimitExceeded);
			}
		}

		if let Some(min_horizontal_scale) = min_horizontal_scale {
			if min_horizontal_scale > 1 {
				return Err(ErrorType::FreeLimitExceeded);
			}
		}

		let volume_size_in_bytes = volume_size as usize * 1024 * 1024 * 1024;
		if volume_size_in_bytes > constants::VOLUME_STORAGE_IN_BYTE {
			return Err(ErrorType::FreeLimitExceeded);
		}
	}

	todo!("Check if any workspace limit for volume is there for user");

	// BEGIN DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut **database)
	.await?;

	if let Some(ports) = ports {
		if ports.is_empty() {
			return Err(ErrorType::WrongParameters);
		}

		// Updating deployment port in database
		query!(
			r#"
			DELETE FROM
				deployment_exposed_port
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		for (port, exposed_port_type) in ports {
			// Adding new exposed port entry to database
			query!(
				r#"
				INSERT INTO 
					deployment_exposed_port(
						deployment_id,
						port,
						port_type
					)
				VALUES
					($1, $2, $3);
				"#,
				deployment_id as _,
				port.value() as i32,
				exposed_port_type as _
			)
			.execute(&mut **database)
			.await?;
		}
	}

	// Updating deployment details
	query!(
		r#"
		UPDATE
			deployment
		SET
			name = COALESCE($1, name),
			machine_type = COALESCE($2, machine_type),
			deploy_on_push = COALESCE($3, deploy_on_push),
			min_horizontal_scale = COALESCE($4, min_horizontal_scale),
			max_horizontal_scale = COALESCE($5, max_horizontal_scale),
			startup_probe_port = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					ELSE
						$6
				END
			),
			startup_probe_path = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					ELSE
						$7
				END
			),
			startup_probe_port_type = (
				CASE
					WHEN $6 = 0 THEN
						NULL
					WHEN $6 IS NULL THEN
						startup_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			),
			liveness_probe_port = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					ELSE
						$8
				END
			),
			liveness_probe_path = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					ELSE
						$9
				END
			),
			liveness_probe_port_type = (
				CASE
					WHEN $8 = 0 THEN
						NULL
					WHEN $8 IS NULL THEN
						liveness_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			)
		WHERE
			id = $10;
		"#,
		name as _,
		machine_type as _,
		deploy_on_push,
		min_horizontal_scale.map(|v| v as i16),
		max_horizontal_scale.map(|v| v as i16),
		startup_probe.map(|probe| probe.port as i32),
		startup_probe.as_ref().map(|probe| probe.path.as_str()),
		liveness_probe.map(|probe| probe.port as i32),
		liveness_probe.as_ref().map(|probe| probe.path.as_str()),
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	// END DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#,
	)
	.execute(&mut **database)
	.await?;

	if let Some(config_mounts) = config_mounts {
		query!(
			r#"
			DELETE FROM
				deployment_config_mounts
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		for (path, file) in &config_mounts {
			query!(
				r#"
				INSERT INTO 
					deployment_config_mounts(
						path,
						file,
						deployment_id
					)
				VALUES
					($1, $2, $3);
				"#,
				path,
				file as &[u8],
				deployment_id as _,
			)
			.execute(&mut **database)
			.await?;
		}
	}

	if let Some(updated_volumes) = volumes {
		let mut current_volumes = query!(
			r#"
				SELECT
					id,
					name,
					deployment_id
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
		.map(|volume| (volume.name.clone(), volume))
		.collect::<BTreeMap<_, _>>();

		for (name, volume) in updated_volumes {
			if let Some(value) = current_volumes.remove(&name) {
				// The new volume is there in the current volumes. Update it
				let current_size = value.volume_size.as_u128() as u16;
				let new_size = volume.size;

				match new_size.cmp(&current_size) {
					Ordering::Less => {
						// Volume size cannot be reduced
						return Err(ErrorType::ReducedVolumeSize);
					}
					Ordering::Equal => (), // Ignore
					Ordering::Greater => {
						query!(
							r#"
							UPDATE 
								deployment_volume
							SET
								volume_size = $1
							WHERE
								name = $2 AND
								deployment_id = $3;
							"#,
							volume_size as i32,
							name,
							deployment_id as _,
						)
						.execute(&mut **database)
						.await?;
					}
				}
			} else {
				// The new volume is not there in the current volumes. Prevent
				// from adding it
				return Err(ErrorType::CannotAddNewVolume);
			}
		}

		if !current_volumes.is_empty() {
			// Preventing removing number of volume
			return Err(ErrorType::CannotRemoveVolume);
		}
	}

	if let Some(environment_variables) = environment_variables {
		query!(
			r#"
			DELETE FROM
				deployment_environment_variable
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		for (key, value) in environment_variables {
			match value {
				EnvironmentVariableValue::String(value) => {
					query!(
						r#"
						INSERT INTO 
							deployment_environment_variable(
								deployment_id,
								name,
								value,
								secret_id
							)
						VALUES
							($1, $2, $3, $4);
						"#,
						deployment_id as _,
						key,
						Some(value),
						None::<Uuid> as _
					)
					.execute(&mut **database)
					.await?;
				}
				EnvironmentVariableValue::Secret {
					from_secret: secret_id,
				} => {
					query!(
						r#"
						INSERT INTO 
							deployment_environment_variable(
								deployment_id,
								name,
								value,
								secret_id
							)
						VALUES
							($1, $2, $3, $4);
						"#,
						deployment_id as _,
						key,
						None::<String>,
						Some(secret_id) as _
					)
					.execute(&mut **database)
					.await?;
				}
			}
		}
	}

	if let Some(new_min_replica) = min_horizontal_scale {
		if new_min_replica != deployment.min_horizontal_scale as u16 {
			for volume in &volumes {
				if todo!("if patr cluster") {
					todo!("stop and start volume usage history");
				};
			}
		}
	}

	let deployment_status = query!(
		r#"
		SELECT
			status as "status: DeploymentStatus"
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
	.map(|deployment| deployment.status)
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	match deployment_status {
		DeploymentStatus::Stopped | DeploymentStatus::Deleted | DeploymentStatus::Created => {
			// Don't update deployments that are explicitly stopped or deleted
		}
		_ => {
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

			if todo!("Deployment on patr region") {
				todo!("Start and stop deployment usage history")
			}
		}
	}

	AppResponse::builder()
		.body(UpdateDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}