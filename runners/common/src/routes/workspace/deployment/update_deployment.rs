use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::{
	app::{AppRequest, ProcessedApiRequest},
	prelude::*,
};

/// Update deployment details. This endpoint is used to update the deployment
/// details. The deployment details that can be updated are the name, machine
/// type, deploy on push, min horizontal scale, max horizontal scale, ports,
/// environment variables, startup probe, liveness probe, config mounts, and
/// volumes. At least one of the values must be updated.
pub async fn update_deployment(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateDeploymentPath {
					workspace_id: _,
					deployment_id,
				},
				query: (),
				headers:
					UpdateDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					UpdateDeploymentRequestProcessed {
						name,
						machine_type,
						deploy_on_push,
						runner: _,
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
		runner_changes_sender: _,
		config: _,
	}: AppRequest<'_, UpdateDeploymentRequest>,
) -> Result<AppResponse<UpdateDeploymentRequest>, ErrorType> {
	info!("Updating deployment: {}", deployment_id);

	// Validate if at least value is to be updated
	if name
		.as_ref()
		.map(|_| 0)
		.or(machine_type.as_ref().map(|_| 0))
		.or(deploy_on_push.as_ref().map(|_| 0))
		.or(min_horizontal_scale.as_ref().map(|_| 0))
		.or(max_horizontal_scale.as_ref().map(|_| 0))
		.or(ports.as_ref().map(|_| 0))
		.or(environment_variables.as_ref().map(|_| 0))
		.or(startup_probe.as_ref().map(|_| 0))
		.or(liveness_probe.as_ref().map(|_| 0))
		.or(config_mounts.as_ref().map(|_| 0))
		.or(volumes.as_ref().map(|_| 0))
		.is_none()
	{
		debug!(
			"No parameters provided for updating deployment: {}",
			deployment_id
		);
		return Err(ErrorType::WrongParameters);
	}

	query(
		r#"
		SELECT
			id
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
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	if let Some(ports) = ports {
		if ports.is_empty() {
			return Err(ErrorType::WrongParameters);
		}

		// Updating deployment port in database
		query(
			r#"
			DELETE FROM
				deployment_exposed_port
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut **database)
		.await?;

		for (port, port_type) in ports {
			query(
				r#"
				INSERT INTO 
					deployment_exposed_port(
						deployment_id,
						port,
						port_type
					)
				VALUES
					(
						$1,
						$2,
						$3
					);
				"#,
			)
			.bind(deployment_id)
			.bind(port.value())
			.bind(port_type.to_string())
			.execute(&mut **database)
			.await?;
		}
	}

	// Updating deployment details
	query(
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
						'http'
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
						'http'
				END
			)
		WHERE
			id = $10;
		"#,
	)
	.bind(name)
	.bind(machine_type)
	.bind(deploy_on_push)
	.bind(min_horizontal_scale)
	.bind(max_horizontal_scale)
	.bind(startup_probe.as_ref().map(|probe| probe.port))
	.bind(startup_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(liveness_probe.as_ref().map(|probe| probe.port))
	.bind(liveness_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(deployment_id)
	.execute(&mut **database)
	.await?;

	if let Some(environment_variables) = environment_variables {
		query(
			r#"
			DELETE FROM
				deployment_environment_variable
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut **database)
		.await?;

		for (name, value) in environment_variables {
			query(
				r#"
				INSERT INTO 
					deployment_environment_variable(
						deployment_id,
						name,
						value,
						secret_id
					)
				VALUES
					(
						$1,
						$2,
						$3,
						$4
					);
				"#,
			)
			.bind(deployment_id)
			.bind(name)
			.bind(value.value())
			.bind(value.secret_id())
			.execute(&mut **database)
			.await?;
		}
	}

	if let Some(config_mounts) = config_mounts {
		query(
			r#"
			DELETE FROM
				deployment_config_mounts
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut **database)
		.await?;

		for (path, file) in config_mounts {
			query(
				r#"
				INSERT INTO 
					deployment_config_mounts(
						deployment_id,
						path,
						file
					)
				VALUES
					(
						$1,
						$2,
						$3
					);
				"#,
			)
			.bind(deployment_id)
			.bind(path)
			.bind(file.into_vec())
			.execute(&mut **database)
			.await?;
		}
	}

	if let Some(updated_volumes) = &volumes {
		query(
			r#"
			DELETE FROM
				deployment_volume_mount
			WHERE
				deployment_id = $1;
			"#,
		)
		.bind(deployment_id)
		.execute(&mut **database)
		.await?;

		for (volume_id, volume_mount_path) in updated_volumes {
			query(
				r#"
				INSERT INTO
					deployment_volume_mount(
						deployment_id,
						volume_id,
						volume_mount_path
					)
				VALUES
					(
						$1,
						$2,
						$3
					);
				"#,
			)
			.bind(deployment_id)
			.bind(volume_id)
			.bind(volume_mount_path.clone())
			.execute(&mut **database)
			.await
			.map_err(|err| match err {
				sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::ResourceInUse,
				sqlx::Error::Database(err) if err.is_foreign_key_violation() => {
					ErrorType::ResourceDoesNotExist
				}
				err => ErrorType::server_error(err),
			})?;
		}
	}

	AppResponse::builder()
		.body(UpdateDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
