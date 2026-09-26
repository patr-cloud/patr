//! Replaces the workspace-volume tables with per-deployment volumes: a volume
//! is now just a path on the deployment, so `deployment_volume` becomes
//! `(deployment_id, path)` and `deployment_volume_mount` goes away.
//!
//! No data is copied. Both tables are provably empty on every runner: nothing
//! has ever inserted a `deployment_volume` row, and with `foreign_keys = ON`
//! every `deployment_volume_mount` insert has failed its FK and rolled back.
//!
//! Drop order is load-bearing: the mount table FKs onto the volume table, and
//! `PRAGMA foreign_keys = OFF` is a no-op inside the transaction the migration
//! runner wraps this in.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	query(
		r#"
		DROP TABLE deployment_volume_mount;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		DROP TABLE deployment_volume;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE deployment_volume(
			deployment_id TEXT NOT NULL,
			path TEXT NOT NULL,

			PRIMARY KEY(deployment_id, path),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
