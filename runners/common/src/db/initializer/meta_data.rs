use crate::prelude::*;

/// Initializes the meta tables
pub async fn initialize_meta_tables(conn: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	info!("Setting up meta tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS meta_data(
			id TEXT PRIMARY KEY,
			value TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *conn)
	.await?;

	Ok(())
}
