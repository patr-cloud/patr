use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	migrate_ci_builds(connection, config).await?;
	Ok(())
}

async fn migrate_ci_builds(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE ci_builds
		ALTER COLUMN author DROP NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
