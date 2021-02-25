use sqlx::{MySql, Transaction};

use crate::query;

pub async fn initialize_drive_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing drive tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS file (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(250) NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS drive_file (
			id BINARY(16) PRIMARY KEY,
			owner_id BINARY(16) NOT NULL,
			folder_id BINARY(16) NOT NULL,
			collection_id BINARY(16) NOT NULL,
			created BIGINT UNSIGNED NOT NULL
		
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_drive_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE file
		ADD CONSTRAINT
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE drive_file
		ADD CONSTRAINT
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}
