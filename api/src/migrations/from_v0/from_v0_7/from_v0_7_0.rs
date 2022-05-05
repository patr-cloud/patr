use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		UPDATE
			secret
		SET
			name = CONCAT('patr-deleted: ', REPLACE(id::TEXT, '-', ''), '@undefined')
		WHERE
			name = CONCAT('patr-deleted-', REPLACE(id::TEXT, '-', ''));
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
