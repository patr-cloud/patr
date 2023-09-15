use crate::prelude::*;

/// Initializes the meta tables
#[instrument(skip(connection))]
pub async fn initialize_meta_tables(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	info!("Initializing meta tables");
	query!(
		r#"
		CREATE TABLE meta_data(
			id VARCHAR(100) CONSTRAINT meta_data_pk PRIMARY KEY,
			value TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

/// Finishes all the meta tables
#[instrument(skip(_connection))]
pub async fn initialize_meta_constraints(_connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	info!("Finishing up meta tables initialization");
	Ok(())
}
