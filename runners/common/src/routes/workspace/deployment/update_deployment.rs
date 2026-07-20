use std::time::Duration;

use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::{actors::runner_supervisor::RunnerSupervisorMessage, app::AppRequest, prelude::*};

/// Update deployment details. This endpoint is used to update the deployment
/// details. The deployment details that can be updated are the name, machine
/// type, deploy on push, min horizontal scale, max horizontal scale, ports,
/// environment variables, startup probe, liveness probe, config mounts, and
/// volumes.
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
						// Self-hosted runners don't support editing the image tag
						// (self-hosted is being deprecated); ignore it.
						image_tag: _,
						machine_type,
						runner: _,
						running_details:
							DeploymentRunningDetails {
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
			},
		database,
		config: _,
		supervisor_ref,
	}: AppRequest<'_, UpdateDeploymentRequest>,
) -> Result<AppResponse<UpdateDeploymentRequest>, ErrorType> {
	info!("Updating deployment: {}", deployment_id);

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

	// Updating deployment details
	query(
		r#"
		UPDATE
			deployment
		SET
			name = $1,
			machine_type = $2,
			deploy_on_push = $3,
			min_horizontal_scale = $4,
			max_horizontal_scale = $5,
			startup_probe_port = $6,
			startup_probe_path = $7,
			startup_probe_port_type = $8,
			liveness_probe_port = $9,
			liveness_probe_path = $10,
			liveness_probe_port_type = $11
		WHERE
			id = $12;
		"#,
	)
	.bind(name)
	.bind(machine_type)
	.bind(deploy_on_push)
	.bind(min_horizontal_scale)
	.bind(max_horizontal_scale)
	.bind(startup_probe.as_ref().map(|probe| probe.port))
	.bind(startup_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(startup_probe.as_ref().map(|_| "http"))
	.bind(liveness_probe.as_ref().map(|probe| probe.port))
	.bind(liveness_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(liveness_probe.as_ref().map(|_| "http"))
	.bind(deployment_id)
	.execute(&mut **database)
	.await?;

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

	for (volume_id, volume_mount_path) in volumes {
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
		.bind(volume_mount_path)
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

	supervisor_ref.send_after(Duration::from_millis(50), move || {
		RunnerSupervisorMessage::UpsertResource {
			resource_id: deployment_id,
			resource_type: ResourceType::Deployment,
		}
	});

	AppResponse::builder()
		.body(UpdateDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
