use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_last_verified_column_to_workspace_domain(connection, config).await?;
	Ok(())
}

async fn add_last_verified_column_to_workspace_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace_domain
		ADD COLUMN last_verified BIGINT NOT NULL
		CONSTRAINT
		workspace_domain_chk_size_unsigned
		CHECK(last_verified >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
