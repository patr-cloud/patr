//! Add `repository_id` column to `deployment` table and make `image_name`
//! nullable. PatrRegistry deployments store `repository_id` instead of a
//! pre-resolved `image_name`.

use crate::prelude::*;

/// Add `repository_id` column and make `image_name` nullable for PatrRegistry.
#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	// SQLite doesn't support ALTER TABLE ADD COLUMN with CHECK constraints
	// that reference other columns, so we recreate the table.

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

	// Copy existing data. All existing rows have image_name set (they were
	// pre-resolved), so they'll satisfy the ExternalRegistry branch of the
	// CHECK constraint.
	query(
		r#"
		INSERT INTO deployment_new(
			id, name, registry, image_name, repository_id, image_tag, status,
			min_horizontal_scale, max_horizontal_scale, machine_type,
			deploy_on_push, startup_probe_port, startup_probe_path,
			startup_probe_port_type, liveness_probe_port, liveness_probe_path,
			liveness_probe_port_type, current_live_digest, deleted
		)
		SELECT
			id, name, registry, image_name, NULL, image_tag, status,
			min_horizontal_scale, max_horizontal_scale, machine_type,
			deploy_on_push, startup_probe_port, startup_probe_path,
			startup_probe_port_type, liveness_probe_port, liveness_probe_path,
			liveness_probe_port_type, current_live_digest, deleted
		FROM
			deployment;
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

	Ok(())
}
