use crate::prelude::*;

/// Initializes the volume tables
#[instrument(skip(connection))]
pub async fn initialize_volume_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up volume tables");

	query(
		r#"
		CREATE TABLE deployment_volume(
			id UUID NOT NULL PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			volume_size INT NOT NULL CHECK(volume_size > 0),
			deleted DATETIME
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query(
		r#"
		CREATE TABLE deployment_volume_mount(
			deployment_id UUID NOT NULL,
			volume_id UUID NOT NULL,
			volume_mount_path TEXT NOT NULL,

			PRIMARY KEY(deployment_id, volume_id),
			FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			FOREIGN KEY(volume_id) REFERENCES deployment_volume(id)
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
