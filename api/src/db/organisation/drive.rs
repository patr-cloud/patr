use crate::{query, Database};

pub async fn initialize_drive_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
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
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_drive_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up drive tables initialization");
	query!(
		r#"
		ALTER TABLE file
		ADD CONSTRAINT file_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
