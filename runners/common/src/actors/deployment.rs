use std::{collections::BTreeMap, marker::PhantomData, time::Duration};

use models::api::workspace::deployment::*;
use ractor::{Actor, ActorProcessingErr, ActorRef};

use super::resource_supervisor::ResourceSupervisorMessage;
use crate::prelude::*;

/// Messages for the [`DeploymentActor`].
///
/// Each DeploymentActor manages the full lifecycle of a single deployment,
/// comparing desired state (SQLite) against running state (executor) and
/// reconciling any differences.
#[derive(Debug)]
pub enum DeploymentMessage {
	/// Desired state may have changed. Re-read from SQLite and reconcile
	/// against `last_applied`. If nothing changed, this is a no-op.
	ConfigUpdated,
	/// Periodic self-sent timer to poll the executor for current status.
	/// Will be replaced by event-driven `StatusChanged` when the Docker
	/// runner adds its event watcher.
	CheckStatus,
	/// Graceful shutdown. Calls `executor.delete_deployment()` if the
	/// deployment is running, then stops the actor.
	Shutdown,
}

/// How often the actor polls the executor for status (until replaced by
/// event-driven status in a future Docker runner update).
const STATUS_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Arguments passed to [`DeploymentActor::pre_start`] to construct the initial
/// state.
pub struct DeploymentActorArgs<E: RunnerExecutor> {
	/// The UUID of the deployment this actor manages.
	pub deployment_id: Uuid,
	/// Database connection pool for reading desired state from SQLite.
	pub database: sqlx::Pool<DatabaseType>,
	/// Runner configuration (settings, mode, bind address, etc.).
	pub config: RunnerSettings<E::Settings>,
	/// Executor-specific initialized state (e.g. Docker client).
	pub runner_state: E::InitializedState,
	/// Reference to the parent ResourceSupervisor for status reporting.
	pub supervisor_ref: ActorRef<ResourceSupervisorMessage>,
}

/// The mutable state held by a running [`DeploymentActor`].
pub struct DeploymentActorState<E: RunnerExecutor> {
	/// The UUID of the deployment this actor manages.
	pub deployment_id: Uuid,
	/// Database connection pool for reading desired state from SQLite.
	pub database: sqlx::Pool<DatabaseType>,
	/// Executor instance for interacting with the underlying runtime (Docker,
	/// Kubernetes, etc.). Constructed once in `pre_start` and reused for the
	/// lifetime of the actor.
	pub executor: E,
	/// Reference to the parent ResourceSupervisor for status reporting.
	pub supervisor_ref: ActorRef<ResourceSupervisorMessage>,
	/// Last config successfully applied to the executor. Used for change
	/// detection so that duplicate `ConfigUpdated` messages are no-ops.
	pub last_applied: Option<(Deployment, DeploymentRunningDetails)>,
	/// Last status reported to the supervisor. Only sends
	/// `ResourceStatusChanged` when the status actually changes.
	pub last_reported_status: Option<DeploymentStatus>,
}

/// Actor responsible for managing the full lifecycle of a single deployment.
///
/// Each deployment gets its own `DeploymentActor` instance, spawned as a
/// supervised child of the
/// [`ResourceSupervisor`][super::resource_supervisor::ResourceSupervisor].
/// The actor reads desired state from SQLite, compares it against the running
/// state reported by the executor, and reconciles any differences.
pub struct DeploymentActor<E: RunnerExecutor> {
	/// Marker for the executor generic — the actor struct itself is stateless.
	_phantom: PhantomData<E>,
}

impl<E: RunnerExecutor> DeploymentActor<E> {
	/// Creates a new [`DeploymentActor`] instance.
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<E> Actor for DeploymentActor<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	type Arguments = DeploymentActorArgs<E>;
	type Msg = DeploymentMessage;
	type State = DeploymentActorState<E>;

	async fn pre_start(
		&self,
		myself: ActorRef<Self::Msg>,
		args: Self::Arguments,
	) -> Result<Self::State, ActorProcessingErr> {
		// Construct the executor once — reused for the lifetime of this actor.
		let executor = E::new(&args.config, args.runner_state).await;

		// Queue an initial ConfigUpdated so the actor reconciles on startup.
		// This message will be processed after pre_start + post_start complete.
		let _ = myself.send_message(DeploymentMessage::ConfigUpdated);

		// Schedule the first status poll.
		myself.send_after(STATUS_POLL_INTERVAL, || DeploymentMessage::CheckStatus);

		Ok(DeploymentActorState {
			deployment_id: args.deployment_id,
			database: args.database,
			executor,
			supervisor_ref: args.supervisor_ref,
			last_applied: None,
			last_reported_status: None,
		})
	}

	async fn handle(
		&self,
		myself: ActorRef<Self::Msg>,
		message: Self::Msg,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		match message {
			DeploymentMessage::ConfigUpdated => {
				handle_config_updated(state).await?;
			}
			DeploymentMessage::CheckStatus => {
				match state
					.executor
					.get_deployment_status(state.deployment_id)
					.await
				{
					Ok(running_status) => {
						handle_status_reconciliation(&myself, state, running_status).await?;
					}
					Err(err) => {
						warn!(
							deployment_id = %state.deployment_id,
							%err,
							"Failed to poll deployment status"
						);
					}
				}
				// Reschedule the next poll.
				myself.send_after(STATUS_POLL_INTERVAL, || DeploymentMessage::CheckStatus);
			}
			DeploymentMessage::Shutdown => {
				// Check if the deployment is actually running before trying to
				// delete it.
				if let Ok(
					DeploymentStatus::Running |
					DeploymentStatus::Deploying |
					DeploymentStatus::Errored,
				) = state
					.executor
					.get_deployment_status(state.deployment_id)
					.await && let Err(err) =
					state.executor.delete_deployment(state.deployment_id).await
				{
					warn!(
						deployment_id = %state.deployment_id,
						%err,
						"Failed to delete deployment during shutdown"
					);
				}

				myself.stop(Some("shutdown requested".to_string()));
			}
		}
		Ok(())
	}
}

/// Handle a `ConfigUpdated` message: re-read desired state from SQLite, compare
/// with `last_applied`, and call the executor if the config has changed.
async fn handle_config_updated<E>(
	state: &mut DeploymentActorState<E>,
) -> Result<(), ActorProcessingErr>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let deployment_id = state.deployment_id;

	let (desired_deployment, desired_details) =
		match get_local_deployment_info(&state.database, deployment_id).await {
			Ok(info) => info,
			Err(RunnerError::UpstreamServerError(ErrorType::ResourceDoesNotExist)) => {
				// Deployment not in DB — possible commit race or already deleted.
				// No-op; periodic Reconcile will catch it.
				trace!("ConfigUpdated but deployment not found in SQLite, skipping");
				return Ok(());
			}
			Err(err) => return Err(err.into()),
		};

	// Compare with last applied config, ignoring the status field since
	// status changes don't represent config changes that need an upsert.
	let config_changed =
		state
			.last_applied
			.as_ref()
			.is_none_or(|(applied_dep, applied_details)| {
				let mut desired_cmp = desired_deployment.clone();
				let mut applied_cmp = applied_dep.clone();

				// Make the status the same so that it doesn't affect the equality check.
				// We are only checking if the config changed, not the status. Status is managed
				// by the executor and can change without a config change, so we ignore it in
				// the comparison.
				desired_cmp.status = DeploymentStatus::Running;
				applied_cmp.status = DeploymentStatus::Running;

				desired_cmp != applied_cmp || *applied_details != desired_details
			});

	if config_changed {
		info!("Desired config changed, upserting deployment");
		if let Err(err) = state
			.executor
			.upsert_deployment(
				WithId::new(deployment_id, desired_deployment.clone()),
				desired_details.clone(),
			)
			.await
		{
			error!(
				deployment_id = %deployment_id,
				%err,
				"Failed to upsert deployment"
			);
			// Report errored status to supervisor, then let the error
			// propagate to kill this actor. The supervisor will respawn
			// with exponential backoff to avoid a tight retry loop.
			let _ = state.supervisor_ref.send_message(
				ResourceSupervisorMessage::ResourceStatusChanged {
					resource_id: deployment_id,
					resource_type: models::rbac::ResourceType::Deployment,
					status: DeploymentStatus::Errored,
				},
			);
			return Err(err.into());
		}
		state.last_applied = Some((desired_deployment, desired_details));
	}

	Ok(())
}

/// Reconcile a running status against the desired status in SQLite. This
/// implements the same state machine as the original
/// `handle_deployment_with_error` (lines 154-261 of
/// `resource_executor/deployment.rs`).
async fn handle_status_reconciliation<E>(
	myself: &ActorRef<DeploymentMessage>,
	state: &mut DeploymentActorState<E>,
	running_status: DeploymentStatus,
) -> Result<(), ActorProcessingErr>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let deployment_id = state.deployment_id;

	let desired_status = match get_local_deployment_status(&state.database, deployment_id).await {
		Ok(status) => status,
		Err(RunnerError::UpstreamServerError(ErrorType::ResourceDoesNotExist)) => {
			// Deployment gone from DB — delete it from the executor and stop.
			info!(
				%deployment_id,
				"Deployment not found in SQLite during status check, cleaning up"
			);
			let _ = state.executor.delete_deployment(deployment_id).await;
			myself.stop(Some("deployment deleted from database".to_string()));
			return Ok(());
		}
		Err(err) => return Err(err.into()),
	};

	debug!(
		%deployment_id,
		%running_status,
		%desired_status,
		"Reconciling deployment status"
	);

	match (running_status, desired_status) {
		// Statuses match — nothing to do.
		(DeploymentStatus::Deploying, DeploymentStatus::Deploying) |
		(DeploymentStatus::Errored, DeploymentStatus::Errored) |
		(DeploymentStatus::Running, DeploymentStatus::Running) |
		(DeploymentStatus::Stopped, DeploymentStatus::Stopped) |
		(DeploymentStatus::Unreachable, DeploymentStatus::Unreachable) => {}

		// DB says unreachable but deployment is something else — update DB
		// to reflect reality.
		(running_status, DeploymentStatus::Unreachable) => {
			info!(
				%deployment_id,
				%running_status,
				"DB says unreachable but deployment is {running_status}, updating"
			);

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
			.await
			.map_err(|err| -> ActorProcessingErr { Box::new(err) })?;

			report_status_if_changed(state, running_status);
		}

		// Running status diverges from desired — update DB to match reality.
		(DeploymentStatus::Unreachable, _) |
		(DeploymentStatus::Errored, DeploymentStatus::Deploying) |
		(DeploymentStatus::Deploying, DeploymentStatus::Errored) |
		(DeploymentStatus::Deploying | DeploymentStatus::Errored, DeploymentStatus::Running) |
		(DeploymentStatus::Running, DeploymentStatus::Deploying | DeploymentStatus::Errored) => {
			info!(
				%deployment_id,
				%running_status,
				"Updating database status to match running state"
			);

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

			report_status_if_changed(state, running_status);
		}

		// DB says stopped but deployment is running/deploying/errored — stop it.
		(
			DeploymentStatus::Deploying | DeploymentStatus::Running | DeploymentStatus::Errored,
			DeploymentStatus::Stopped,
		) => {
			info!("DB says stopped, deleting deployment");
			state.executor.delete_deployment(deployment_id).await?;
		}

		// Deployment is stopped but DB says it should be running — start it.
		(DeploymentStatus::Stopped, DeploymentStatus::Deploying | DeploymentStatus::Running) |
		(DeploymentStatus::Stopped, DeploymentStatus::Errored) => {
			info!("Deployment stopped but should be running, upserting");

			let (deployment, running_details) =
				get_local_deployment_info(&state.database, deployment_id).await?;

			if let Err(err) = state
				.executor
				.upsert_deployment(
					WithId::new(deployment_id, deployment.clone()),
					running_details.clone(),
				)
				.await
			{
				error!("Failed to start deployment");
				let _ = state.supervisor_ref.send_message(
					ResourceSupervisorMessage::ResourceStatusChanged {
						resource_id: deployment_id,
						resource_type: models::rbac::ResourceType::Deployment,
						status: DeploymentStatus::Errored,
					},
				);
				return Err(err.into());
			}
			state.last_applied = Some((deployment, running_details));
		}
	}

	Ok(())
}

/// Send a `ResourceStatusChanged` message to the supervisor, but only if the
/// status actually changed since the last report.
fn report_status_if_changed<E: RunnerExecutor>(
	state: &mut DeploymentActorState<E>,
	status: DeploymentStatus,
) {
	if state.last_reported_status.as_ref() != Some(&status) {
		state.last_reported_status = Some(status);
		let _ =
			state
				.supervisor_ref
				.send_message(ResourceSupervisorMessage::ResourceStatusChanged {
					resource_id: state.deployment_id,
					resource_type: models::rbac::ResourceType::Deployment,
					status,
				});
	}
}

// ---------------------------------------------------------------------------
// SQLite helpers — these will move to a shared db module in Phase 5.
// ---------------------------------------------------------------------------

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
		.ok_or_else(|| {
			ErrorType::server_error("corrupted deployment, cannot find environment variable value")
		})?;

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
			repository_id,
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
	let registry_url = row.try_get::<String, _>("registry")?;
	let image_name = row.try_get::<Option<String>, _>("image_name")?;
	let repository_id = row.try_get::<Option<Uuid>, _>("repository_id")?;
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

	let registry = match (repository_id, image_name) {
		(Some(repository_id), _) => DeploymentRegistry::PatrRegistry {
			registry: models::api::workspace::deployment::PatrRegistry,
			repository_id,
		},
		(None, Some(image_name)) => DeploymentRegistry::ExternalRegistry {
			registry: registry_url,
			image_name,
		},
		(None, None) => {
			return Err(ErrorType::server_error(
				"corrupted deployment: neither repository_id nor image_name is set",
			)
			.into());
		}
	};

	Ok((
		Deployment {
			name,
			image_tag,
			status,
			registry,
			// Dummy runner ID — no runner-id concept in self-hosted PATR
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
