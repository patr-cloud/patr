use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	reset_invalid_birthday(&mut *connection, config).await?;
	Ok(())
}

pub(super) async fn reset_invalid_birthday(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	log::info!("Updating all invalid user dob");

	query!(
		r#"
		UPDATE
			"user"
		SET
			dob = NULL
		WHERE
			dob IS NOT NULL AND
			dob > (NOW() - INTERVAL '13 YEARS');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	log::info!("All invalid dobs updated");

	query!(
		r#"
		ALTER TABLE "user" ADD CONSTRAINT user_chk_dob_is_13_plus CHECK (dob IS NULL OR dob < (NOW() - INTERVAL '13 YEARS'));
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
