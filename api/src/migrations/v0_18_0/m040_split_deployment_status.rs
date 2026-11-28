//! Splits `deployment.status` into what the user asked for and what the runner
//! sees, so the two stop overwriting each other.
//!
//! - `should_run` is the intent. Only the API writes it, on create, start and stop.
//! - `observed_status` is what the runner last reported. Only the runner writes it, and it's reset
//!   to NULL when the deployment moves to another runner.
//!
//! `status` is dropped, `DEPLOYMENT_STATUS` is recreated with the public values,
//! and `status` comes back as a column generated from the two. Whether the
//! runner is online can't be stored, since it changes with time, so reads
//! override `status` with `unreachable` when `IS_RUNNER_ONLINE` says the runner
//! isn't.
//!
//! The backfill compares `status::TEXT`, so a value this migration doesn't
//! expect maps to NULL instead of failing the cast. `created`, `pushed` and
//! `deleted` were never reported by a runner, so they become NULL too.

use crate::prelude::*;

#[macros::migration]
async fn migrate(
	connection: &mut DatabaseConnection,
	_config: &AppConfig,
) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TYPE OBSERVED_DEPLOYMENT_STATUS AS ENUM(
			'deploying',
			'running',
			'crashing',
			'health_check_failing',
			'image_pull_failed',
			'invalid_architecture',
			'errored',
			'stopped'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment
			ADD COLUMN should_run BOOLEAN,
			ADD COLUMN observed_status OBSERVED_DEPLOYMENT_STATUS;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE
			deployment
		SET
			should_run = status::TEXT NOT IN ('stopped', 'deleted'),
			observed_status = CASE
				WHEN status::TEXT IN ('deploying', 'running', 'stopped', 'errored') THEN
					status::TEXT::OBSERVED_DEPLOYMENT_STATUS
				ELSE
					NULL
			END;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment
			ALTER COLUMN should_run SET NOT NULL,
			DROP COLUMN status;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE DEPLOYMENT_STATUS;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TYPE DEPLOYMENT_STATUS AS ENUM(
			'deploying',
			'running',
			'crashing',
			'health_check_failing',
			'image_pull_failed',
			'invalid_architecture',
			'errored',
			'stopped',
			'stopping',
			'unreachable'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment
			ADD COLUMN status DEPLOYMENT_STATUS NOT NULL GENERATED ALWAYS AS (
				CASE
					WHEN NOT should_run AND (
						observed_status IS NULL OR
						observed_status = 'stopped'
					) THEN 'stopped'::DEPLOYMENT_STATUS
					WHEN NOT should_run THEN 'stopping'::DEPLOYMENT_STATUS
					WHEN (
						observed_status IS NULL OR
						observed_status = 'stopped'
					) THEN 'deploying'::DEPLOYMENT_STATUS
					WHEN observed_status = 'deploying' THEN 'deploying'::DEPLOYMENT_STATUS
					WHEN observed_status = 'running' THEN 'running'::DEPLOYMENT_STATUS
					WHEN observed_status = 'crashing' THEN 'crashing'::DEPLOYMENT_STATUS
					WHEN observed_status = 'health_check_failing' THEN 'health_check_failing'::DEPLOYMENT_STATUS
					WHEN observed_status = 'image_pull_failed' THEN 'image_pull_failed'::DEPLOYMENT_STATUS
					WHEN observed_status = 'invalid_architecture' THEN 'invalid_architecture'::DEPLOYMENT_STATUS
					WHEN observed_status = 'errored' THEN 'errored'::DEPLOYMENT_STATUS
				END
			) STORED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE FUNCTION IS_RUNNER_ONLINE(
			is_connected BOOLEAN,
			last_seen TIMESTAMPTZ
		) RETURNS BOOLEAN AS $$
			SELECT is_connected AND COALESCE(last_seen > NOW() - INTERVAL '120 seconds', FALSE);
		$$ LANGUAGE SQL STABLE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
