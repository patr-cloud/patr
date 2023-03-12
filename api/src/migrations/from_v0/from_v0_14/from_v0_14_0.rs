use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_processed_to_static_site_uploads(connection, config).await?;

	Ok(())
}

pub async fn add_processed_to_static_site_uploads(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE static_site_upload_history
		ADD COLUMN processed TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			static_site_upload_history
		SET
			processed = created;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
