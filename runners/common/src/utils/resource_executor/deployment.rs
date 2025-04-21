use std::{collections::BTreeMap, time::Duration};

use models::api::workspace::deployment::*;
use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// The resource executor task that will be used to specifically manage the
/// deployments of the runner.
pub(super) async fn handle_deployment<E>(
	deployment_id: Uuid,
	state: AppState<E>,
	cancellation_token: CancellationToken,
) where
	E: RunnerExecutor + Send + 'static,
{
	loop {
		let executor = E::new(&state.config, state.runner_state.clone()).await;
		let result = handle_deployment_with_error(
			deployment_id,
			executor,
			state.clone(),
			&cancellation_token,
		)
		.await;

		if let Err(RunnerError::ExitSignalReceived) = result {
			// If the task was cancelled, we need to stop the task
			// and return an error
			return;
		}

		// Try again in a second
		time::sleep(Duration::from_secs(1)).await;
	}
}

async fn handle_deployment_with_error<E>(
	deployment_id: Uuid,
	executor: E,
	state: AppState<E>,
	cancellation_token: &CancellationToken,
) -> Result<!, RunnerError>
where
	E: RunnerExecutor + Send + 'static,
{
	loop {
		// Keep checking for the status of the deployment and
		// update the database

		let status = executor
			.get_deployment_status(deployment_id)
			.with_cancel_check_of(&cancellation_token)
			.await??;

		let (deployment, running_details) =
			get_local_deployment_info(&state.database, deployment_id).await?;

		if deployment.status == DeploymentStatus::Running ||
			deployment.status == DeploymentStatus::Deploying
		{
			executor
				.upsert_deployment(WithId::new(deployment_id, deployment), running_details)
				.await?;
		}

		// Update the status of the deployment in the database
		query(
			r#"
			UPDATE
				deployment
			SET
				status = $1
			WHERE
				id = $2;
			"#,
		)
		.bind(status.to_string())
		.bind(deployment_id)
		.execute(&state.database)
		.await?;

		if cancellation_token.is_cancelled() {
			return Err(RunnerError::ExitSignalReceived);
		}
	}
}

async fn get_local_deployment_info(
	database: &sqlx::Pool<DatabaseType>,
	deployment_id: Uuid,
) -> Result<(Deployment, DeploymentRunningDetails), RunnerError> {
	let ports = query(
		r#"
		SELECT
			port,
			port_type
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.fetch_all(database)
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
	.fetch_all(database)
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
	.fetch_all(database)
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
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| {
		let volume_id = row.try_get::<Uuid, _>("volume_id")?;
		let volume_mount_path = row.try_get::<String, _>("volume_mount_path")?;

		Ok((volume_id, volume_mount_path))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let row = query(
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
	.fetch_one(database)
	.await
	.map_err(|err| match err {
		sqlx::Error::RowNotFound => ErrorType::ResourceDoesNotExist,
		err => err.into(),
	})?;

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

	let startup_probe = row
		.try_get::<Option<u16>, _>("startup_probe_port")?
		.zip(row.try_get::<Option<String>, _>("startup_probe_path")?)
		.map(|(port, path)| DeploymentProbe { port, path });

	let liveness_probe = row
		.try_get::<Option<u16>, _>("liveness_probe_port")?
		.zip(row.try_get::<Option<String>, _>("liveness_probe_path")?)
		.map(|(port, path)| DeploymentProbe { port, path });

	Ok((
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
	))
}
