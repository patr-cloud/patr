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
		ALTER TABLE transaction
		DROP CONSTRAINT transaction_description_check;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE transaction
		ADD CONSTRAINT transaction_description_check CHECK(
			(
				transaction_type = 'credits' AND
				description IS NOT NULL
			) OR (
				transaction_type != 'credits'
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
