//! Name all CHECK constraints across deployment-related tables and remove
//! the `deployment_update_log` table and its triggers (replaced by actor
//! messages). SQLite doesn't support renaming constraints in-place, so each
//! table is recreated.

use crate::prelude::*;

/// Name all CHECK constraints and remove the defunct trigger/update_log
/// infrastructure.
#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	// Disable foreign keys so we can freely drop/recreate tables regardless
	// of reference order.
	query("PRAGMA foreign_keys = OFF;")
		.execute(&mut *connection)
		.await?;

	// Drop triggers (no longer needed — actor messages replace them)

	query("DROP TRIGGER IF EXISTS deployment_tg_before_insert_update_log;")
		.execute(&mut *connection)
		.await?;
	query("DROP TRIGGER IF EXISTS deployment_tg_before_update_update_log;")
		.execute(&mut *connection)
		.await?;
	query("DROP TRIGGER IF EXISTS deployment_tg_before_delete_update_log;")
		.execute(&mut *connection)
		.await?;
	query("DROP TABLE IF EXISTS deployment_update_log;")
		.execute(&mut *connection)
		.await?;

	// Recreate deployment_exposed_port with named constraints

	query(
		r#"
		CREATE TABLE deployment_exposed_port_new(
			deployment_id TEXT NOT NULL,
			port INTEGER NOT NULL,
			port_type TEXT NOT NULL
				CONSTRAINT deployment_exposed_port_chk_port_type_enum
				CHECK(port_type IN ('http')),

			PRIMARY KEY(deployment_id, port, port_type),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			CONSTRAINT deployment_exposed_port_chk_port_range
				CHECK(port > 0 AND port <= 65535)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		INSERT INTO deployment_exposed_port_new
		SELECT * FROM deployment_exposed_port;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query("DROP TABLE deployment_exposed_port;")
		.execute(&mut *connection)
		.await?;

	query("ALTER TABLE deployment_exposed_port_new RENAME TO deployment_exposed_port;")
		.execute(&mut *connection)
		.await?;

	// Recreate deployment with named constraints

	query(
		r#"
		CREATE TABLE deployment_new(
			id TEXT NOT NULL PRIMARY KEY,
			name TEXT NOT NULL,
			registry TEXT NOT NULL,
			image_name TEXT,
			repository_id TEXT,
			image_tag TEXT NOT NULL,
			status TEXT NOT NULL,
			min_horizontal_scale INTEGER NOT NULL,
			max_horizontal_scale INTEGER NOT NULL,
			machine_type TEXT NOT NULL,
			deploy_on_push BOOLEAN NOT NULL,
			startup_probe_port INTEGER,
			startup_probe_path TEXT,
			startup_probe_port_type TEXT
				CONSTRAINT deployment_chk_startup_probe_port_type_enum
				CHECK(startup_probe_port_type IN ('http')),
			liveness_probe_port INTEGER,
			liveness_probe_path TEXT,
			liveness_probe_port_type TEXT
				CONSTRAINT deployment_chk_liveness_probe_port_type_enum
				CHECK(liveness_probe_port_type IN ('http')),
			current_live_digest TEXT,
			deleted DATETIME,

			CONSTRAINT deployment_chk_registry_image_name_repository_id_exclusivity CHECK(
				(
					registry = 'registry.patr.cloud' AND
					repository_id IS NOT NULL AND
					image_name IS NULL
				) OR (
					registry != 'registry.patr.cloud' AND
					image_name IS NOT NULL AND
					repository_id IS NULL
				)
			),

			CONSTRAINT deployment_chk_status_enum CHECK(
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

			CONSTRAINT deployment_chk_min_horizontal_scale_range CHECK(
				min_horizontal_scale >= 0 AND
				min_horizontal_scale <= 256 AND
				min_horizontal_scale <= max_horizontal_scale
			),

			CONSTRAINT deployment_chk_max_horizontal_scale_range CHECK(
				max_horizontal_scale >= 0 AND
				max_horizontal_scale <= 256 AND
				max_horizontal_scale >= min_horizontal_scale
			),

			CONSTRAINT deployment_chk_image_name_nonempty
				CHECK(LENGTH(TRIM(image_name)) > 0),
			CONSTRAINT deployment_chk_image_tag_nonempty
				CHECK(LENGTH(TRIM(image_tag)) > 0),

			CONSTRAINT deployment_chk_startup_probe_cohesion CHECK(
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

			CONSTRAINT deployment_chk_liveness_probe_cohesion CHECK(
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
		INSERT INTO deployment_new
		SELECT * FROM deployment;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query("DROP TABLE deployment;")
		.execute(&mut *connection)
		.await?;

	query("ALTER TABLE deployment_new RENAME TO deployment;")
		.execute(&mut *connection)
		.await?;

	// Recreate deployment_environment_variable with named constraints

	query(
		r#"
		CREATE TABLE deployment_environment_variable_new(
			deployment_id TEXT NOT NULL,
			name TEXT NOT NULL,
			value TEXT,
			secret_id TEXT,

			PRIMARY KEY(deployment_id, name),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			CONSTRAINT deployment_environment_variable_chk_name_nonempty
				CHECK(LENGTH(TRIM(name)) > 0),
			CONSTRAINT deployment_environment_variable_chk_value_nonempty
				CHECK(LENGTH(TRIM(value)) > 0),
			CONSTRAINT deployment_environment_variable_chk_value_secret_id_required
				CHECK(value IS NOT NULL OR secret_id IS NOT NULL)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		INSERT INTO deployment_environment_variable_new
		SELECT * FROM deployment_environment_variable;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query("DROP TABLE deployment_environment_variable;")
		.execute(&mut *connection)
		.await?;

	query(
		"ALTER TABLE deployment_environment_variable_new RENAME TO deployment_environment_variable;",
	)
	.execute(&mut *connection)
	.await?;

	// ── Recreate deployment_volume with named constraint ──

	query(
		r#"
		CREATE TABLE deployment_volume_new(
			id UUID NOT NULL PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			volume_size INT NOT NULL
				CONSTRAINT deployment_volume_chk_volume_size_positive CHECK(volume_size > 0),
			deleted DATETIME
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		INSERT INTO deployment_volume_new
		SELECT * FROM deployment_volume;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query("DROP TABLE deployment_volume;")
		.execute(&mut *connection)
		.await?;

	query("ALTER TABLE deployment_volume_new RENAME TO deployment_volume;")
		.execute(&mut *connection)
		.await?;

	// Re-enable foreign keys

	query("PRAGMA foreign_keys = ON;")
		.execute(&mut *connection)
		.await?;

	Ok(())
}
