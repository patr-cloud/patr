use crate::{migrate_query as query, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	alter_user_login_table(&mut *connection).await?;

	Ok(())
}

async fn alter_user_login_table(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE user_login
		ALTER COLUMN login_id TYPE UUID
        USING CAST(ENCODE(login_id, 'hex') AS UUID);
		"#
	)
	.execute(connection)
	.await?;

	Ok(())
}
