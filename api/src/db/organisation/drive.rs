use sqlx::Transaction;

use crate::{query, Database};

pub async fn initialize_drive_pre(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing drive tables");
	query!(
		r#"
		CREATE TABLE file (
			id BYTEA CONSTRAINT file_pk PRIMARY KEY,
			name VARCHAR(250) NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_drive_post(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up drive tables initialization");
	query!(
		r#"
		ALTER TABLE file
		ADD CONSTRAINT file_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}
