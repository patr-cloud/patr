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
	add_attempts_for_password_reset(connection, config).await?;
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

async fn add_attempts_for_password_reset(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE password_reset_request
		ADD COLUMN attempts INT NOT NULL DEFAULT 0
			CONSTRAINT password_reset_request_attempts_non_negative
				CHECK(attempts >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE password_reset_request
		ALTER COLUMN attempts DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
