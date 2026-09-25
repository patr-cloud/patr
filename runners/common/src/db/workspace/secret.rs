use crate::prelude::*;

/// Initializes the secret tables.
#[instrument(skip(connection))]
pub async fn initialize_secret_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up secret tables");

	query(
		r#"
		CREATE TABLE secret(
			id TEXT NOT NULL PRIMARY KEY,
			last_updated DATETIME NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
