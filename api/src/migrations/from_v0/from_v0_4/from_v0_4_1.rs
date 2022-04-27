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
			deployment
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_database
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
