//! Keeps every deployment on a runner in its own workspace. Until now only
//! `runner` was a foreign key, so a deployment could be created on, or moved
//! to, another workspace's runner.
//!
//! `runner(id, workspace_id)` becomes unique so a composite foreign key can
//! point at it. It replaces `deployment_fk_runner`: both columns are NOT NULL,
//! so it already guarantees the runner exists, and a missing runner then fails
//! the same constraint as one in another workspace. Deployments that already
//! break the rule are logged by ID first, and then adding the key fails.

use crate::prelude::*;

#[macros::migration]
async fn migrate(
	connection: &mut DatabaseConnection,
	_config: &AppConfig,
) -> Result<(), ErrorType> {
	let violators = sqlx::query_scalar::<_, Uuid>(
		r#"
		SELECT
			deployment.id
		FROM
			deployment
		INNER JOIN
			runner
		ON
			runner.id = deployment.runner
		WHERE
			runner.workspace_id != deployment.workspace_id;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?;

	if !violators.is_empty() {
		error!(
			"Deployments on a runner in another workspace: {}",
			violators
				.iter()
				.map(ToString::to_string)
				.collect::<Vec<_>>()
				.join(", ")
		);
	}

	sqlx::query(
		r#"
		ALTER TABLE runner
		ADD CONSTRAINT runner_uq_id_workspace_id UNIQUE(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment
		DROP CONSTRAINT deployment_fk_runner,
		ADD CONSTRAINT deployment_fk_runner_workspace_id
		FOREIGN KEY(runner, workspace_id) REFERENCES runner(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
