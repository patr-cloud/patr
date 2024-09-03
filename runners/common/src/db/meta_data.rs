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
			id TEXT CONSTRAINT meta_data_pk PRIMARY KEY,
			value TEXT NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

/// Initializes the meta table indices
#[instrument(skip(_connection))]
pub async fn initialize_meta_indices(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up meta tables indices");
	Ok(())
}

/// Initializes the meta tables constraints
#[instrument(skip(_connection))]
pub async fn initialize_meta_constraints(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up meta tables constraints");
	Ok(())
}
