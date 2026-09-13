use crate::prelude::*;

/// Initializes the volume tables
#[instrument(skip(connection))]
pub async fn initialize_volume_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up volume tables");

	// No path CHECK here: the API validates paths before they reach a runner.
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

/// Initializes the volume indices
#[instrument(skip(_connection))]
pub async fn initialize_volume_indices(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up volume indices");

	Ok(())
}
