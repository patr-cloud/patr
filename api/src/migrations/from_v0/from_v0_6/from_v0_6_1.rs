use crate::{migrate_query as query, utils::settings::Settings, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE managed_url
		ALTER CONSTRAINT managed_url_fk_deployment_id_port
		SET DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
