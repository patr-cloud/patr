use std::error::Error;

use semver::Version;

// use sqlx::query;
use crate::prelude::*;

mod deployment;
mod meta_data;

pub async fn intialize(app: &AppState) -> Result<(), Box<dyn Error>> {
	info!("Initializing database");

	let tables = query!(
		r#"
		SELECT
			*
		FROM
			sqlite_schema
		WHERE
			type = 'table'
		;
		"#,
	)
	.fetch_all(&app.database)
	.await?;

	let mut transaction = app.database.begin().await?;

	if tables.is_empty() {
		warn!("No tables exist. Creating fresh");

		meta_data::initialize_meta_tables(&mut transaction).await?;
		deployment::initialize_deployment_tables(&mut transaction).await?;

		query!(
			r#"
				INSERT INTO meta_data(id, value)
				VALUES
					('version_major', $1),
					('version_minor', $2),
					('version_patch', $3)
				;
			"#,
			constants::DATABASE_VERSION.major.to_string(),
			constants::DATABASE_VERSION.minor.to_string(),
			constants::DATABASE_VERSION.patch.to_string()
		)
		.execute(&mut transaction)
		.await?;

		transaction.commit().await?;

		info!("Database created");

		return Ok(());
	} else {
		// TODO: Write the migration code here after figuring out how to get
		// sqlx to not throw errors

		todo!();
		// If it already exists, perform a migration with the known values

		// let rows = query!(
		// 	r#"
		// 	SELECT
		// 		*
		// 	FROM
		// 		meta_data
		// 	WHERE
		// 		id = 'version_major' OR
		// 		id = 'version_minor' OR
		// 		id = 'version_patch';
		// 	"#
		// )
		// .fetch_all(&mut *transaction)
		// .await?;
		//
		// let mut version = Version::new(0, 0, 0);
	}

	Ok(())
}
