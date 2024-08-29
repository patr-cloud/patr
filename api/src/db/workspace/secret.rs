use crate::prelude::*;

/// Initializes the secret tables
#[instrument(skip(connection))]
pub async fn initialize_secret_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up secret tables");
	query!(
		r#"
		CREATE TABLE secret(
			id UUID NOT NULL,
			name CITEXT NOT NULL,
			workspace_id UUID NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the secret indices
#[instrument(skip(connection))]
pub async fn initialize_secret_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up secret indices");
	query!(
		r#"
		ALTER TABLE secret
		ADD CONSTRAINT secret_pk
		PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			secret_uq_workspace_id_name
		ON
			secret(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the secret constraints
#[instrument(skip(connection))]
pub async fn initialize_secret_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up secret constraints");
	query!(
		r#"
		ALTER TABLE secret
			ADD CONSTRAINT secret_chk_name_is_trimmed CHECK(name = TRIM(name)),
			ADD CONSTRAINT secret_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, owner_id, deleted)
					DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
