use crate::prelude::*;

/// Initializes the deployment tables
#[instrument(skip(connection))]
pub async fn initialize_deployment_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up deployment tables");

	query(
		r#"
		CREATE TABLE deployment_machine_type(
			id TEXT NOT NULL PRIMARY KEY,
			cpu_count INTEGER NOT NULL,
			memory_count INTEGER NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// TODO: Move this somewhere else, this is just here for testing
	query(
		r#"
		INSERT INTO
			deployment_machine_type(
				id,
				cpu_count,
				memory_count
			)
		VALUES
			($1, 1, 1024);
		"#,
	)
	.bind(Uuid::new_v4().to_string())
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE deployment(
			id TEXT NOT NULL PRIMARY KEY,
			name TEXT NOT NULL,
			registry TEXT NOT NULL,
			image_name TEXT NOT NULL,
			image_tag TEXT NOT NULL,
			status TEXT NOT NULL,
			min_horizontal_scale INTEGER NOT NULL,
			max_horizontal_scale INTEGER NOT NULL,
			machine_type TEXT NOT NULL,
			deploy_on_push BOOLEAN NOT NULL,
			startup_probe_port INTEGER,
			startup_probe_path TEXT,
			startup_probe_port_type TEXT CHECK(
				startup_probe_port_type IN ('http')
			),
			liveness_probe_port INTEGER,
			liveness_probe_path TEXT,
			liveness_probe_port_type TEXT CHECK(
				liveness_probe_port_type IN ('http')
			),
			current_live_digest TEXT,
			deleted DATETIME,

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
				min_horizontal_scale <= 256 AND
				min_horizontal_scale <= max_horizontal_scale
			),

			CHECK(
				max_horizontal_scale >= 0 AND
				max_horizontal_scale <= 256 AND
				max_horizontal_scale >= min_horizontal_scale
			),

			CHECK(LENGTH(TRIM(image_name)) > 0),
			CHECK(LENGTH(TRIM(image_tag)) > 0),

			CHECK(
				( 
					startup_probe_port IS NULL AND
					startup_probe_path IS NULL AND
					startup_probe_port_type IS NULL
				) OR (
					startup_probe_port IS NOT NULL AND
					startup_probe_path IS NOT NULL AND
					startup_probe_port_type IS NOT NULL
				)
			),

			CHECK(
				( 
					liveness_probe_port IS NULL AND
					liveness_probe_path IS NULL AND
					liveness_probe_port_type IS NULL
				) OR (
					liveness_probe_port IS NOT NULL AND
					liveness_probe_path IS NOT NULL AND
					liveness_probe_port_type IS NOT NULL
				)
			),

			FOREIGN KEY(machine_type) REFERENCES deployment_machine_type(id),
			FOREIGN KEY(id, startup_probe_port, startup_probe_port_type)
				REFERENCES deployment_exposed_port(deployment_id, port, port_type),
			FOREIGN KEY(id, liveness_probe_port, liveness_probe_port_type)
				REFERENCES deployment_exposed_port(deployment_id, port, port_type)
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
			value TEXT,
			secret_id TEXT,

			PRIMARY KEY(deployment_id, name),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			CHECK(LENGTH(TRIM(name)) > 0),
			CHECK(LENGTH(TRIM(value)) > 0),
			CHECK(value IS NOT NULL OR secret_id IS NOT NULL)
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
			port_type TEXT NOT NULL CHECK(port_type IN ('http')),

			PRIMARY KEY(deployment_id, port, port_type),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			CHECK(port > 0 AND port <= 65535)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE deployment_config_mounts(
			deployment_id TEXT NOT NULL,
			path TEXT NOT NULL,
			file BLOB NOT NULL,

			PRIMARY KEY(deployment_id, path),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE deployment_deploy_history(
			deployment_id TEXT NOT NULL,
			image_digest TEXT NOT NULL,
			registry TEXT NOT NULL,
			image_tag TEXT NOT NULL,
			image_name TEXT NOT NULL,
			created DATETIME NOT NULL,

			PRIMARY KEY(deployment_id, image_digest),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id)
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
