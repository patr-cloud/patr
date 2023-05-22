use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_is_frozen_column_to_workspace(connection, config).await?;
	add_survey_table(connection, config).await?;

	Ok(())
}

async fn add_is_frozen_column_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN is_frozen BOOLEAN NOT NULL DEFAULT FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN is_frozen DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_survey_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE SURVEY(
			user_id UUID NOT NULL
				CONSTRAINT survey_fk_user_id REFERENCES "user"(id),
			version TEXT,
			response JSON NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
