use crate::prelude::*;

/// Initializes the volume tables
#[instrument(skip(connection))]
pub async fn initialize_volume_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up volume tables");

	query!(
		r#"
		CREATE TABLE deployment_volume(
			deployment_id UUID NOT NULL,
			path TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the volume indices
#[instrument(skip(connection))]
pub async fn initialize_volume_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up volume indices");
	query!(
		r#"
		ALTER TABLE deployment_volume
			ADD CONSTRAINT deployment_volume_pk PRIMARY KEY(deployment_id, path);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the volume constraints
#[instrument(skip(connection))]
pub async fn initialize_volume_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up volume constraints");

	// The path must be absolute and normalized: `/`-separated non-empty
	// segments, no trailing slash, no `.` or `..` segments. Docker would reject
	// anything else at deploy time; this rejects it at create time.
	query!(
		r#"
		ALTER TABLE deployment_volume
			ADD CONSTRAINT deployment_volume_fk_deployment_id
				FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			ADD CONSTRAINT deployment_volume_chk_path_valid CHECK(
				path ~ '^(/[^/]+)+$' AND
				path !~ '/\.\.?(/|$)' AND
				LENGTH(path) <= 4096
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
