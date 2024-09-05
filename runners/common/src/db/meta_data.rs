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
