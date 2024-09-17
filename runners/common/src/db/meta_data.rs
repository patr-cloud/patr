use crate::prelude::*;

/// Initializes the meta tables
#[instrument(skip(connection))]
pub async fn initialize_meta_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up meta tables");
	query(
		r#"
		CREATE TABLE meta_data(
			id TEXT NOT NULL PRIMARY KEY,
			value TEXT NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

/// Initializes the meta indices
#[instrument(skip(_connection))]
pub async fn initialize_meta_indices(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up meta indices");
	Ok(())
}
