use api_models::utils::Uuid;
use sqlx::Row;

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
	let users = query!(
		r#"
		SELECT
			*
		FROM
			"user"
		WHERE
			dob IS NOT NULL AND
			dob > (now() - interval '13 years');
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.get::<Uuid, _>("id"))
	.collect::<Vec<_>>();

	let dob_to_be_updated = users.len();

	for (index, user_id) in users.into_iter().enumerate() {
		log::info!(
			"Setting user dob {}/{} to null",
			index + 1,
			dob_to_be_updated
		);
		query!(
			r#"
			UPDATE
				"user"
			SET
				dob = NULL
			WHERE
				id = $1;
			"#,
			user_id
		)
		.execute(&mut *connection)
		.await?;
	}
	log::info!("All invalid dobs updated");

	query!(
		r#"
		ALTER TABLE "user" 
		ADD CONSTRAINT user_chk_dob_is_13_plus 
			CHECK (dob IS NULL OR dob < (now() - interval '13 years'));
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
