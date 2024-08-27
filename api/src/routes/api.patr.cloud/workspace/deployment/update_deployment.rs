use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// Update deployment details. This endpoint is used to update the deployment
/// details. The deployment details that can be updated are the name, machine
/// type, deploy on push, min horizontal scale, max horizontal scale, ports,
/// environment variables, startup probe, liveness probe, config mounts, and
/// volumes. At least one of the values must be updated.
pub async fn update_deployment(
	AuthenticatedAppRequest {
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
						runner,
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
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, UpdateDeploymentRequest>,
) -> Result<AppResponse<UpdateDeploymentRequest>, ErrorType> {
	info!("Updating deployment: {}", deployment_id);

	// Validate if at least value is to be updated
	if name
		.as_ref()
		.map(|_| 0)
		.or(machine_type.as_ref().map(|_| 0))
		.or(deploy_on_push.as_ref().map(|_| 0))
		.or(runner.as_ref().map(|_| 0))
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

	query!(
		r#"
		SELECT
			id
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

		query!(
			r#"
			INSERT INTO 
				deployment_exposed_port(
					deployment_id,
					port,
					port_type
				)
			VALUES
				(
					UNNEST($1::UUID[]),
					UNNEST($2::INTEGER[]),
					UNNEST($3::EXPOSED_PORT_TYPE[])
				);
			"#,
			&ports
				.iter()
				.map(|_| deployment_id.into())
				.collect::<Vec<_>>(),
			&ports
				.iter()
				.map(|(port, _)| port.value() as i32)
				.collect::<Vec<_>>(),
			&ports
				.iter()
				.map(|(_, port_type)| port_type.to_string())
				.collect::<Vec<String>>() as _,
		)
		.execute(&mut **database)
		.await?;
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
			runner = COALESCE($4, runner),
			min_horizontal_scale = COALESCE($5, min_horizontal_scale),
			max_horizontal_scale = COALESCE($6, max_horizontal_scale),
			startup_probe_port = (
				CASE
					WHEN $7 = 0 THEN
						NULL
					ELSE
						$7
				END
			),
			startup_probe_path = (
				CASE
					WHEN $7 = 0 THEN
						NULL
					ELSE
						$8
				END
			),
			startup_probe_port_type = (
				CASE
					WHEN $7 = 0 THEN
						NULL
					WHEN $7 IS NULL THEN
						startup_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			),
			liveness_probe_port = (
				CASE
					WHEN $9 = 0 THEN
						NULL
					ELSE
						$9
				END
			),
			liveness_probe_path = (
				CASE
					WHEN $9 = 0 THEN
						NULL
					ELSE
						$10
				END
			),
			liveness_probe_port_type = (
				CASE
					WHEN $9 = 0 THEN
						NULL
					WHEN $9 IS NULL THEN
						liveness_probe_port_type
					ELSE
						'http'::EXPOSED_PORT_TYPE
				END
			)
		WHERE
			id = $11;
		"#,
		name as _,
		machine_type as _,
		deploy_on_push,
		runner as _,
		min_horizontal_scale.map(|v| v as i16),
		max_horizontal_scale.map(|v| v as i16),
		startup_probe.as_ref().map(|probe| probe.port as i32),
		startup_probe.as_ref().map(|probe| probe.path.as_str()),
		liveness_probe.as_ref().map(|probe| probe.port as i32),
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
				(
					UNNEST($1::UUID[]),
					UNNEST($2::TEXT[]),
					UNNEST($3::TEXT[]),
					UNNEST($4::UUID[])
				);
			"#,
			&environment_variables
				.iter()
				.map(|_| deployment_id.into())
				.collect::<Vec<sqlx::types::Uuid>>(),
			&environment_variables
				.iter()
				.map(|(name, _)| name.clone())
				.collect::<Vec<_>>(),
			&environment_variables
				.iter()
				.map(|(_, value)| value.value().cloned())
				.collect::<Vec<Option<String>>>() as _,
			&environment_variables
				.iter()
				.map(|(_, value)| value.secret_id().map(Into::into))
				.collect::<Vec<Option<sqlx::types::Uuid>>>() as _,
		)
		.execute(&mut **database)
		.await?;
	}

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

		query!(
			r#"
			INSERT INTO 
				deployment_config_mounts(
					deployment_id,
					path,
					file
				)
			VALUES
				(
					UNNEST($1::UUID[]),
					UNNEST($2::TEXT[]),
					UNNEST($3::BYTEA[])
				);
			"#,
			&config_mounts
				.iter()
				.map(|_| deployment_id.into())
				.collect::<Vec<_>>(),
			&config_mounts
				.iter()
				.map(|(path, _)| path.clone())
				.collect::<Vec<_>>(),
			&config_mounts
				.iter()
				.map(|(_, file)| file.to_vec())
				.collect::<Vec<_>>(),
		)
		.execute(&mut **database)
		.await?;
	}

	if let Some(updated_volumes) = &volumes {
		query!(
			r#"
			DELETE FROM
				deployment_volume_mount
			WHERE
				deployment_id = $1;
			"#,
			deployment_id as _,
		)
		.execute(&mut **database)
		.await?;

		query!(
			r#"
			INSERT INTO
				deployment_volume_mount(
					deployment_id,
					volume_id,
					volume_mount_path
				)
			VALUES
				(
					UNNEST($1::UUID[]),
					UNNEST($2::UUID[]),
					UNNEST($3::TEXT[])
				);
			"#,
			&updated_volumes
				.iter()
				.map(|_| deployment_id.into())
				.collect::<Vec<_>>(),
			&updated_volumes
				.iter()
				.map(|(volume_id, _)| (*volume_id).into())
				.collect::<Vec<_>>(),
			&updated_volumes
				.iter()
				.map(|(_, volume_mount_path)| volume_mount_path.clone())
				.collect::<Vec<_>>(),
		)
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

	AppResponse::builder()
		.body(UpdateDeploymentResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
