use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_oauth_column_to_user_table(&mut *connection, config).await?;
	add_oauth_column_to_user_to_sign_up_table(&mut *connection, config).await?;

	Ok(())
}

async fn add_oauth_column_to_user_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE "user"
			ADD COLUMN is_oauth_user BOOLEAN DEFAULT false;
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

async fn add_oauth_column_to_user_to_sign_up_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE user_to_sign_up
			ADD COLUMN is_oauth_user BOOLEAN DEFAULT false;
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
