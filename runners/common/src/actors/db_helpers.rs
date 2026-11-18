//! Shared database utility functions for creating/deleting deployments in
//! SQLite. Used by both the WebSocketActor and HTTP route handlers.
//!
//! These are pure database functions — no API calls, no runner config needed.

use std::collections::BTreeMap;

use models::api::workspace::deployment::*;
use time::OffsetDateTime;

use crate::prelude::*;

/// Insert or update a deployment and all its related data in the local SQLite
/// database.
///
/// Stores the raw `DeploymentRegistry` data (including `repository_id` for
/// PatrRegistry) without resolving image names. Resolution happens at upsert
/// time in the DeploymentActor / executor.
///
/// Owned child tables (`deployment_exposed_port`,
/// `deployment_environment_variable`, `deployment_config_mounts`,
/// `deployment_volume`) are cleared and re-inserted; `managed_url` and
/// `deployment_deploy_history` are NOT touched so a `DeploymentUpdated` event
/// from upstream doesn't cascade-wipe the runner's managed URL state.
#[instrument(skip_all, fields(deployment_id = %deployment_id))]
pub async fn upsert_deployment_in_database(
	connection: &mut DatabaseConnection,
	WithId {
		id: deployment_id,
		data:
			Deployment {
				name,
				registry,
				image_tag,
				status: _,
				should_run,
				runner: _,
				machine_type,
				current_live_digest,
			},
	}: WithId<Deployment>,
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
	}: DeploymentRunningDetails,
) -> Result<(), RunnerError> {
	trace!(
		"Upserting deployment in database with ID: {}",
		deployment_id
	);

	// Insert with NULL probe FKs (they reference `deployment_exposed_port` rows
	// we're about to recreate). On conflict, also clear probes so the delete
	// below can drop the old `deployment_exposed_port` rows without violating
	// the FK; we re-set them at the end once the new rows exist.
	query!(
		r#"
		INSERT INTO
			deployment(
				id,
				name,
				registry,
				image_name,
				repository_id,
				image_tag,
				should_run,
				machine_type,
				min_horizontal_scale,
				max_horizontal_scale,
				deploy_on_push,
				startup_probe_port,
				startup_probe_path,
				startup_probe_port_type,
				liveness_probe_port,
				liveness_probe_path,
				liveness_probe_port_type,
				current_live_digest,
				deleted
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7,
				$8,
				$9,
				$10,
				$11,
				NULL,
				NULL,
				NULL,
				NULL,
				NULL,
				NULL,
				$12,
				NULL
			)
		ON CONFLICT(id) DO UPDATE SET
			name = excluded.name,
			registry = excluded.registry,
			image_name = excluded.image_name,
			repository_id = excluded.repository_id,
			image_tag = excluded.image_tag,
			should_run = excluded.should_run,
			machine_type = excluded.machine_type,
			min_horizontal_scale = excluded.min_horizontal_scale,
			max_horizontal_scale = excluded.max_horizontal_scale,
			deploy_on_push = excluded.deploy_on_push,
			startup_probe_port = NULL,
			startup_probe_path = NULL,
			startup_probe_port_type = NULL,
			liveness_probe_port = NULL,
			liveness_probe_path = NULL,
			liveness_probe_port_type = NULL,
			current_live_digest = excluded.current_live_digest,
			deleted = excluded.deleted;
		"#,
		deployment_id,
		name.to_string(),
		registry.registry_url(),
		registry.image_name(),
		registry.repository_id(),
		image_tag.to_string(),
		should_run,
		machine_type,
		min_horizontal_scale,
		max_horizontal_scale,
		deploy_on_push,
		current_live_digest
	)
	.execute(&mut *connection)
	.await?;

	// Clear owned child rows so we can re-insert the latest set. Probes were
	// nulled above, so `deployment_exposed_port` is no longer referenced from
	// `deployment` and can be deleted safely.
	query!(
		r#"
		DELETE FROM
			deployment_volume
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		DELETE FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	for (port, port_type) in &ports {
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
			deployment_id,
			port.value(),
			port_type
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		UPDATE
			deployment
		SET
			startup_probe_port = $2,
			startup_probe_path = $3,
			startup_probe_port_type = $4,
			liveness_probe_port = $5,
			liveness_probe_path = $6,
			liveness_probe_port_type = $7
		WHERE
			id = $1;
		"#,
		deployment_id,
		startup_probe.as_ref().map(|probe| probe.port),
		startup_probe.as_ref().map(|probe| probe.path.as_str()),
		startup_probe.as_ref().map(|_| ExposedPortType::Http),
		liveness_probe.as_ref().map(|probe| probe.port),
		liveness_probe.as_ref().map(|probe| probe.path.as_str()),
		liveness_probe.as_ref().map(|_| ExposedPortType::Http)
	)
	.execute(&mut *connection)
	.await?;

	for (name, value) in &environment_variables {
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
			deployment_id,
			name,
			value.value(),
			value.secret_id()
		)
		.execute(&mut *connection)
		.await?;
	}

	for (path, file) in &config_mounts {
		query!(
			r#"
			INSERT INTO
				deployment_config_mounts(
					deployment_id,
					path,
					file
				)
			VALUES
				($1, $2, $3);
			"#,
			deployment_id,
			path,
			file.to_vec()
		)
		.execute(&mut *connection)
		.await?;
	}

	for path in volumes.keys() {
		query!(
			r#"
			INSERT INTO
				deployment_volume(
					deployment_id,
					path
				)
			VALUES
				($1, $2);
			"#,
			deployment_id,
			path
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

/// Insert or replace a managed URL row. The host string is the resolved
/// FQDN (e.g. `myapp.example.com`) — the WebSocket / resync layer resolves
/// `domain_id` upstream before calling this.
#[instrument(skip(connection))]
pub async fn upsert_managed_url_in_database(
	connection: &mut DatabaseConnection,
	managed_url_id: Uuid,
	host: &str,
	path: &str,
	deployment_id: Uuid,
	port: u16,
) -> Result<(), RunnerError> {
	query!(
		r#"
		INSERT INTO managed_url(
			id,
			host,
			path,
			deployment_id,
			port
		)
		VALUES ($1, $2, $3, $4, $5)
		ON CONFLICT(id) DO UPDATE SET
			host = excluded.host,
			path = excluded.path,
			deployment_id = excluded.deployment_id,
			port = excluded.port;
		"#,
		managed_url_id,
		host,
		path,
		deployment_id,
		port as i64
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Delete a managed URL row by ID.
#[instrument(skip(connection))]
pub async fn delete_managed_url_in_database(
	connection: &mut DatabaseConnection,
	managed_url_id: Uuid,
) -> Result<(), RunnerError> {
	query!(
		r#"
		DELETE FROM
			managed_url
		WHERE
			id = $1;
		"#,
		managed_url_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Insert or update when a secret's value last changed.
#[instrument(skip(connection))]
pub async fn upsert_secret_in_database(
	connection: &mut DatabaseConnection,
	secret_id: Uuid,
	last_updated: OffsetDateTime,
) -> Result<(), RunnerError> {
	query!(
		r#"
		INSERT INTO
			secret(
				id,
				last_updated
			)
		VALUES
			($1, $2)
		ON CONFLICT(id) DO UPDATE SET
			last_updated = excluded.last_updated;
		"#,
		secret_id,
		last_updated
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Delete a deployment and all its related data from the local SQLite database.
///
/// This is a free-function version of the old
/// `Runner::delete_deployment_in_database`.
#[instrument(skip(connection))]
pub async fn delete_deployment_in_database(
	connection: &mut DatabaseConnection,
	deployment_id: Uuid,
) -> Result<(), RunnerError> {
	// Clear referencing managed URLs first — the FK on managed_url.deployment_id
	// would otherwise block the deployment delete (foreign_keys = ON).
	query!(
		r#"
		DELETE FROM
			managed_url
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_volume
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	// Clear probes first, the same way the upsert does: they reference the
	// `deployment_exposed_port` rows deleted next.
	query!(
		r#"
		UPDATE
			deployment
		SET
			startup_probe_port = NULL,
			startup_probe_path = NULL,
			startup_probe_port_type = NULL,
			liveness_probe_port = NULL,
			liveness_probe_path = NULL,
			liveness_probe_port_type = NULL
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Get the deployment and its running details from the local database.
pub async fn get_local_deployment_info(
	database: &sqlx::Pool<DatabaseType>,
	deployment_id: Uuid,
) -> Result<(Deployment, DeploymentRunningDetails), RunnerError> {
	let ports = query!(
		r#"
		SELECT
			port,
			port_type
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| (StringifiedU16::new(row.port), row.port_type))
	.collect::<BTreeMap<_, _>>();

	let environment_variables = query!(
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
		deployment_id
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|env| {
		let value = env.value.map(EnvironmentVariableValue::String);
		let secret_id = env
			.secret_id
			.map(|from_secret| EnvironmentVariableValue::Secret { from_secret });

		let value = match (value, secret_id) {
			(Some(value), None) => Some(value),
			(None, Some(secret)) => Some(secret),
			_ => None,
		}
		.ok_or_else(|| {
			ErrorType::server_error("corrupted deployment, cannot find environment variable value")
		})?;

		Ok((env.name, value))
	})
	.collect::<Result<BTreeMap<_, _>, ErrorType>>()?;

	let config_mounts = query!(
		r#"
		SELECT
			path,
			file
		FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| (row.path, Base64String::from(row.file)))
	.collect::<BTreeMap<_, _>>();

	let volumes = query!(
		r#"
		SELECT
			path
		FROM
			deployment_volume
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| (row.path, VolumeConfig {}))
	.collect::<BTreeMap<_, _>>();

	let row = query!(
		r#"
		SELECT
			id,
			name,
			registry,
			image_name,
			repository_id,
			image_tag,
			should_run,
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
		deployment_id
	)
	.fetch_one(database)
	.await
	.map_err(|err| match err {
		sqlx::Error::RowNotFound => ErrorType::ResourceDoesNotExist,
		err => err.into(),
	})?;

	let name = row.name;
	let image_tag = row.image_tag;
	let should_run = row.should_run;
	let registry_url = row.registry;
	let machine_type = row.machine_type;
	let current_live_digest = row.current_live_digest;

	let deploy_on_push = row.deploy_on_push;
	let min_horizontal_scale = row.min_horizontal_scale;
	let max_horizontal_scale = row.max_horizontal_scale;

	let startup_probe = row
		.startup_probe_port
		.zip(row.startup_probe_path)
		.map(|(port, path)| DeploymentProbe { port, path });

	let liveness_probe = row
		.liveness_probe_port
		.zip(row.liveness_probe_path)
		.map(|(port, path)| DeploymentProbe { port, path });

	let registry = match (row.repository_id, row.image_name) {
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
			// Placeholder — the public status the API derives isn't stored in the
			// local database, only `observed_status` is
			status: DeploymentStatus::Observed(ObservedDeploymentStatus::Unknown),
			should_run,
			registry,
			// Dummy runner ID — the runner isn't stored in the local database
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

/// Load the runner's whole local state: every deployment with its running
/// details, every managed URL and every secret version.
pub async fn load_local_state(
	database: &sqlx::Pool<DatabaseType>,
) -> Result<LocalState, RunnerError> {
	let deployment_ids = query!(
		r#"
		SELECT
			id
		FROM
			deployment;
		"#
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| row.id)
	.collect::<Vec<_>>();

	let mut deployments = BTreeMap::new();
	for deployment_id in deployment_ids {
		deployments.insert(
			deployment_id,
			get_local_deployment_info(database, deployment_id).await?,
		);
	}

	let managed_urls = query!(
		r#"
		SELECT
			id,
			host,
			path,
			deployment_id,
			port
		FROM
			managed_url;
		"#
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.id,
			LocalManagedUrl {
				host: row.host,
				path: row.path,
				deployment_id: row.deployment_id,
				port: row.port,
			},
		)
	})
	.collect::<BTreeMap<_, _>>();

	let secrets = query!(
		r#"
		SELECT
			id,
			last_updated
		FROM
			secret;
		"#
	)
	.fetch_all(database)
	.await?
	.into_iter()
	.map(|row| (row.id, row.last_updated))
	.collect::<BTreeMap<_, _>>();

	Ok(LocalState {
		deployments,
		managed_urls,
		secrets,
	})
}
