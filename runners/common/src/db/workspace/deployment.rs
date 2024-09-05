use crate::prelude::*;

/// Initializes the deployment tables
#[instrument(skip(connection))]
pub async fn initialize_deployment_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment tables");

	query(
		r#"
		CREATE TABLE machine_type(
			id TEXT PRIMARY KEY,
			cpu_count INTEGER NOT NULL,
			memory_count INTEGER NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE IF NOT EXISTS deployment(
			id TEXT NOT NULL PRIMARY KEY,
			name TEXT NOT NULL,
			registry TEXT NOT NULL DEFAULT 'docker.io',
			image_name TEXT NOT NULL,
			image_tag TEXT NOT NULL,
			status TEXT NOT NULL DEFAULT 'created',
			min_horizontal_scale INTEGER NOT NULL DEFAULT 1,
			max_horizontal_scale INTEGER NOT NULL DEFAULT 1,
			machine_type TEXT NOT NULL,
			deploy_on_push BOOLEAN NOT NULL DEFAULT FALSE,
			startup_probe_port INTEGER,
			startup_probe_path TEXT,
			startup_probe_port_type TEXT CHECK(liveness_probe_port_type IN ('tcp', 'http')),
			liveness_probe_port INTEGER,
			liveness_probe_path TEXT,
			liveness_probe_port_type TEXT CHECK (liveness_probe_port_type IN ('tcp', 'http')),
			current_live_digest TEXT,

			CHECK( 
				status IN (
					'created', 
					'pushed', 
					'deploying', 
					'running', 
					'stopped', 
					'errored', 
					'deleted'
				) 
			),

			CHECK(
				min_horizontal_scale >= 0 AND
				min_horizontal_scale <= max_horizontal_scale
				min_horizontal_scale <= 256
			),

			CHECK(
				max_horizontal_scale >= 0 AND
				max_horizontal_scale <= 256
				max_horizontal_scale >= min_horizontal_scale
			),

			CHECK (LENGTH(TRIM(image_name)) > 0),
			CHECK (LENGTH(TRIM(image_tag)) > 0),

			CHECK (
				( 
					startup_probe_port IS NULL AND
					startup_probe_path IS NULL AND
					startup_probe_port_type IS NULL
				) OR (
					startup_probe_port IS NOT NULL AND
					startup_probe_path IS NOT NULL AND
					startup_probe_port_type NOT IS NULL
				)
			),

			CHECK (
				( 
					liveness_probe_port IS NULL AND
					liveness_probe_path IS NULL AND
					liveness_probe_port_type IS NULL
				) OR (
					liveness_probe_port IS NOT NULL AND
					liveness_probe_path IS NOT NULL AND
					liveness_probe_port_type NOT IS NULL
				)
			),

			FOREIGN KEY (machine_type) REFERENCES machine_type(id),
			FOREIGN KEY (id, startup_probe_port, startup_probe_type) REFERENCES deployment_exposed_port(deployment_id, port, port_type)
				DEFERRABLE INITIALLY IMMEDIATE,

			FOREIGN KEY (id, liveness_probe_port, liveness_probe_type) REFERENCES deployment_exposed_port(deployment_id, port, port_type)
				DEFERRABLE INITIALLY IMMEDIATE,
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id TEXT NOT NULL,
			name TEXT NOT NULL,
			value TEXT NOT NULL,
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE deployment_exposed_port(
			deployment_id TEXT NOT NULL,
			port INTEGER NOT NULL,
			port_type TEXT CHECK (port_type IN ('http')),
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE deployment_config_mounts(
			path TEXT NOT NULL,
			file BLOB NOT NULL,
			deployment_id TEXT NOT NULL

			FOREIGN KEY (deployment_id) REFERENCES deployment(id)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the deployment indices
#[instrument(skip(_connection))]
pub async fn initialize_deployment_indices(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment indices");

	Ok(())
}
