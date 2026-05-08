//! Shared database utility functions for creating/deleting deployments in
//! SQLite. Used by both the WebSocketActor and HTTP route handlers.
//!
//! These are pure database functions — no API calls, no runner config needed.

use models::api::workspace::deployment::*;

use crate::prelude::*;

/// Create a deployment and all its related data in the local SQLite database.
///
/// Stores the raw `DeploymentRegistry` data (including `repository_id` for
/// PatrRegistry) without resolving image names. Resolution happens at upsert
/// time in the DeploymentActor / executor.
#[instrument(skip(connection))]
pub async fn create_deployment_in_database(
	connection: &mut DatabaseConnection,
	WithId {
		id: deployment_id,
		data:
			Deployment {
				name,
				registry,
				image_tag,
				status,
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
	trace!("Creating deployment in database with ID: {}", deployment_id);

	query(
		r#"
		INSERT INTO
			deployment(
				id,
				name,
				registry,
				image_name,
				repository_id,
				image_tag,
				status,
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
			);
		"#,
	)
	.bind(deployment_id)
	.bind(name.to_string())
	.bind(registry.registry_url())
	.bind(registry.image_name())
	.bind(registry.repository_id())
	.bind(image_tag.to_string())
	.bind(status)
	.bind(machine_type)
	.bind(min_horizontal_scale)
	.bind(max_horizontal_scale)
	.bind(deploy_on_push)
	.bind(current_live_digest)
	.execute(&mut *connection)
	.await?;

	for (port, port_type) in &ports {
		query(
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
		)
		.bind(deployment_id)
		.bind(port.value())
		.bind(port_type)
		.execute(&mut *connection)
		.await?;
	}

	query(
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
	)
	.bind(deployment_id)
	.bind(startup_probe.as_ref().map(|probe| probe.port))
	.bind(startup_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(startup_probe.as_ref().map(|_| ExposedPortType::Http))
	.bind(liveness_probe.as_ref().map(|probe| probe.port))
	.bind(liveness_probe.as_ref().map(|probe| probe.path.as_str()))
	.bind(liveness_probe.as_ref().map(|_| ExposedPortType::Http))
	.execute(&mut *connection)
	.await?;

	for (name, value) in &environment_variables {
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
				($1, $2, $3, $4);
			"#,
		)
		.bind(deployment_id)
		.bind(name)
		.bind(value.value())
		.bind(value.secret_id())
		.execute(&mut *connection)
		.await?;
	}

	for (path, file) in &config_mounts {
		query(
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
		)
		.bind(deployment_id)
		.bind(path)
		.bind(file.to_vec())
		.execute(&mut *connection)
		.await?;
	}

	for (volume_id, mount_path) in &volumes {
		query(
			r#"
			INSERT INTO
				deployment_volume_mount(
					deployment_id,
					volume_id,
					volume_mount_path
				)
			VALUES
				($1, $2, $3);
			"#,
		)
		.bind(deployment_id)
		.bind(volume_id)
		.bind(mount_path)
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
	query(
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
	)
	.bind(managed_url_id)
	.bind(host)
	.bind(path)
	.bind(deployment_id)
	.bind(port as i64)
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
	query(
		r#"
		DELETE FROM
			managed_url
		WHERE
			id = $1;
		"#,
	)
	.bind(managed_url_id)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Truncate the managed URL table — used at the start of a full resync.
#[instrument(skip(connection))]
pub async fn delete_all_managed_urls_in_database(
	connection: &mut DatabaseConnection,
) -> Result<(), RunnerError> {
	query(
		r#"
		DELETE FROM
			managed_url;
		"#,
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
	query(
		r#"
		DELETE FROM
			managed_url
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		DELETE FROM
			deployment_volume_mount
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		DELETE FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		DELETE FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut *connection)
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
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		DELETE FROM
			deployment
		WHERE
			id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
