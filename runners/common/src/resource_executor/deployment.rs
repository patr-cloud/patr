use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures::future::{self, Either};
use models::api::workspace::deployment::*;
use tokio::{sync::Notify, time};
use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// The resource executor task that will be used to specifically manage the
/// deployments of the runner.
pub(super) async fn handle_deployment<E>(
	deployment_id: Uuid,
	state: AppState<E>,
	cancellation_token: CancellationToken,
	update_notifier: Arc<Notify>,
) where
	E: RunnerExecutor + Send + 'static,
{
	loop {
		let executor = E::new(&state.config, state.runner_state.clone()).await;
		let Err(error) = handle_deployment_with_error(
			deployment_id,
			executor,
			state.clone(),
			&cancellation_token,
			&update_notifier,
		)
		.await;

		if let RunnerError::UpstreamServerError(ErrorType::ResourceDoesNotExist) |
		RunnerError::ExitSignalReceived = error
		{
			// If the task was cancelled or if the deployment was deleted, we need to stop
			// the task and return an error
			return;
		}
		// If the task was not cancelled, we need to log the error
		// and continue the loop
		error!("Error while handling deployment: {}", error);

		// Try again in a second
		time::sleep(Duration::from_secs(1)).await;
	}
}

/// Handle the deployment with the given ID. This will keep checking the status
/// of the deployment and update the database accordingly. If the deployment is
/// deleted, this function will return. If the exit signal is received, this
/// function will return.
async fn handle_deployment_with_error<E>(
	deployment_id: Uuid,
	executor: E,
	state: AppState<E>,
	cancellation_token: &CancellationToken,
	update_notifier: &Notify,
) -> Result<!, RunnerError>
where
	E: RunnerExecutor + Send + 'static,
{
	let mut get_deployment_status_future = Box::pin(executor.next_deployment_status(deployment_id));
	let mut update_notifier_future = Box::pin(update_notifier.notified());

	loop {
		// Keep checking for the status of the deployment and update the database
		trace!("Checking deployment status for {}", deployment_id);

		let running_status = future::select(get_deployment_status_future, update_notifier_future)
			.with_cancel_check_of(cancellation_token)
			.await?;

		// What is the status right now?
		let running_status = match running_status {
			Either::Left((status, future)) => {
				// The running deployment has changed. Check what's on the db and update
				// accordingly.

				get_deployment_status_future =
					Box::pin(executor.next_deployment_status(deployment_id));
				update_notifier_future = future;

				status
			}
			Either::Right(((), future)) => {
				// The notifier told us that the db has changed. Get the current running status
				// and check that against the db

				get_deployment_status_future = future;
				update_notifier_future = Box::pin(update_notifier.notified());

				executor.get_deployment_status(deployment_id).await
			}
		}?;

		// What is it supposed to be as per the db?
		// If it's deleted, it'll return an ErrorType::ResourceDoesNotExist, and that's
		// handled above to stop the task
		let deployment_status = get_local_deployment_status(&state.database, deployment_id).await?;

		debug!(
			"Deployment {} is currently {} but is supposed to be {} as per the database",
			deployment_id, running_status, deployment_status
		);

		// TODO is this really the right way?
		match (running_status, deployment_status) {
			(DeploymentStatus::Deploying, DeploymentStatus::Deploying) |
			(DeploymentStatus::Errored, DeploymentStatus::Errored) |
			(DeploymentStatus::Running, DeploymentStatus::Running) |
			(DeploymentStatus::Stopped, DeploymentStatus::Stopped) |
			(DeploymentStatus::Unreachable, DeploymentStatus::Unreachable) => {
				// If the status is the same, we don't need to do anything
				// just continue the loop
				continue;
			}
			(running_status, DeploymentStatus::Unreachable) => {
				// If the db status is unreachable but the deployment is anything else,
				// we need to update the db
				todo!("Update the db to {running_status} along with upstream, if managed");
			}
			(DeploymentStatus::Unreachable, _) |
			(DeploymentStatus::Errored, DeploymentStatus::Deploying) |
			(DeploymentStatus::Deploying, DeploymentStatus::Errored) |
			(
				DeploymentStatus::Deploying | DeploymentStatus::Errored,
				DeploymentStatus::Running,
			) |
			(
				DeploymentStatus::Running,
				DeploymentStatus::Deploying | DeploymentStatus::Errored,
			) => {
				// If the running status is unreachable, we need to update the db regardless of
				// what it currently is
				// OR
				// If the db thinks it's currently running but it's still deploying or errored
				// OR
				// If the db thinks it's errored but the deployment is coming back up
				// OR
				// If the db thinks it's deploying or errored but the deployment is up and
				// running,
				// OR
				// If the deployment is errored, but the db says it's deploying, then:

				info!("Updating database to {}", running_status);

				// force the db to be as per deployment
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
				.bind(running_status.to_string())
				.bind(deployment_id)
				.execute(&state.database)
				.await?;

				// Send status update signal to upstream Patr server to update the status there
				let _ =
					state
						.task_status_sender
						.send(ExecutorStatusUpdate::DeploymentStatusUpdated {
							deployment_id,
							status: running_status,
						});
			}
			(
				DeploymentStatus::Deploying | DeploymentStatus::Running | DeploymentStatus::Errored,
				DeploymentStatus::Stopped,
			) => {
				// If the db says it's stopped, but the deployment is either deploying or
				// running or errored, stop it

				info!("Deleting the deployment");

				executor.delete_deployment(deployment_id).await?;
			}
			(
				DeploymentStatus::Stopped,
				DeploymentStatus::Deploying | DeploymentStatus::Running,
			) |
			(DeploymentStatus::Stopped, DeploymentStatus::Errored) => {
				// If the deployment is stopped, but the db says it's supposed to be deploying
				// or running, we need to start the deployment
				// If the deployment is stopped, but the db says it's supposed to be errored,
				// then the deployment was supposed to be running in the first place (or it
				// couldn't have gotten into an errored state). So start the deployment

				info!("Running the deployment");

				let (deployment, running_details) =
					get_local_deployment_info(&state.database, deployment_id).await?;
				executor
					.upsert_deployment(WithId::new(deployment_id, deployment), running_details)
					.await?;
			}
		}

		if cancellation_token.is_cancelled() {
			return Err(RunnerError::ExitSignalReceived);
		}
	}
}

/// Get the current status of the deployment from the local database.
async fn get_local_deployment_status(
	database: &sqlx::Pool<DatabaseType>,
	deployment_id: Uuid,
) -> Result<DeploymentStatus, RunnerError> {
	let row = query(
		r#"
		SELECT
			status
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

	row.try_get::<DeploymentStatus, _>("status")
		.map_err(Into::into)
}

/// Get the deployment and its running details from the local database.
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
