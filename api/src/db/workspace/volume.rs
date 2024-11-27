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
			id UUID NOT NULL,
			name TEXT NOT NULL,
			volume_size BIGINT NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_volume_mount(
			deployment_id UUID NOT NULL,
			volume_id UUID NOT NULL,
			volume_mount_path TEXT NOT NULL
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
			ADD CONSTRAINT deployment_volume_pk PRIMARY KEY(id),
			ADD CONSTRAINT deployment_volume_uq_name UNIQUE(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_volume_mount
			ADD CONSTRAINT deployment_volume_mount_pk PRIMARY KEY(deployment_id, volume_id),
			ADD CONSTRAINT deployment_volume_mount_fk_volume_id
				FOREIGN KEY(volume_id) REFERENCES deployment_volume(id),
			ADD CONSTRAINT deployment_volume_mount_fk_deployment_id
				FOREIGN KEY(deployment_id) REFERENCES deployment(id);
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
	query!(
		r#"
		ALTER TABLE deployment_volume
		ADD CONSTRAINT deployment_volume_chk_size_unsigned
		CHECK(volume_size > 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
