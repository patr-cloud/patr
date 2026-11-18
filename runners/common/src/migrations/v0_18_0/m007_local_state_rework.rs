//! Splits a deployment's intent from its observed status, mirroring the API:
//! `deployment.status` is replaced by `should_run`, written from the API, and
//! `observed_status`, written by the runner. Also adds `object_finalizer`, and
//! removes the `meta_data` keys the self-hosted mode stored.
//!
//! The migration runner wraps this in a transaction, where `PRAGMA foreign_keys
//! = OFF` is a no-op, and `deployment` is referenced by every child table. So
//! foreign keys are deferred to the commit instead, and `deployment` is copied
//! aside, dropped and recreated under its own name before the rows go back in.
//! Re-inserting the parent rows is what clears the violations the drop leaves
//! on the child rows; renaming a `deployment_new` into place wouldn't.

use crate::prelude::*;

/// Replace `deployment.status` with `should_run` and `observed_status`, add the
/// `object_finalizer` table, and delete the self-hosted `meta_data` keys.
#[macros::migration]
async fn migrate(
	connection: &mut DatabaseConnection,
	_config: &RunnerSettings<()>,
) -> Result<(), sqlx::Error> {
	sqlx::query("PRAGMA defer_foreign_keys = ON;")
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		CREATE TABLE deployment_old AS
		SELECT
			*
		FROM
			deployment;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query("DROP TABLE deployment;")
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		CREATE TABLE deployment(
			id TEXT NOT NULL PRIMARY KEY,
			name TEXT NOT NULL,
			registry TEXT NOT NULL,
			image_name TEXT,
			repository_id TEXT,
			image_tag TEXT NOT NULL,
			should_run BOOLEAN NOT NULL,
			observed_status TEXT,
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

			CONSTRAINT deployment_chk_observed_status_enum CHECK(
				observed_status IN (
					'deploying',
					'running',
					'crashing',
					'health_check_failing',
					'image_pull_failed',
					'invalid_architecture',
					'errored',
					'stopped'
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

	// The old status held the API's status, overwritten with the observed one
	// by the deployment actor, which never wrote `stopped` itself. So `stopped`
	// only ever meant the user stopped it, matching the API's own backfill.
	sqlx::query(
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
				current_live_digest,
				deleted
			)
		SELECT
			id,
			name,
			registry,
			image_name,
			repository_id,
			image_tag,
			status NOT IN ('stopped', 'deleted'),
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
			current_live_digest,
			deleted
		FROM
			deployment_old;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query("DROP TABLE deployment_old;")
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		CREATE TABLE object_finalizer(
			object_id TEXT NOT NULL,
			deployment_id TEXT NOT NULL,
			finalizer TEXT NOT NULL,
			state TEXT NOT NULL,
			attempts INTEGER NOT NULL,
			last_error TEXT,
			last_updated DATETIME NOT NULL,

			PRIMARY KEY(object_id, finalizer),

			CONSTRAINT object_finalizer_chk_finalizer_enum CHECK(
				finalizer IN (
					'volume'
				)
			),

			CONSTRAINT object_finalizer_chk_state_enum CHECK(
				state IN (
					'pending',
					'running',
					'done',
					'failed'
				)
			),

			CONSTRAINT object_finalizer_chk_last_error_enum CHECK(
				last_error IN (
					'api_unreachable',
					'still_on_this_runner'
				)
			)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			meta_data
		WHERE
			id IN (
				'user_id',
				'password_hash',
				'first_name',
				'last_name'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
