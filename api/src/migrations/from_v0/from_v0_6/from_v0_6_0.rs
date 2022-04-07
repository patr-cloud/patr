use api_models::utils::Uuid;
use semver::Version;

use crate::{
	migrate_query as query,
	models::rbac,
	utils::settings::Settings,
	Database,
};

async fn migrate_from_v0_6_0(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	user_to_workspace_permissions(connection, config).await?;
	
	Ok(())
}

pub async fn user_to_workspace_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {

	for &permission in [
		"workspace::delete",
		"workspace::user::createUser",
		"workspace::user::deleteUser",
		"workspace::user::updateUser",
	]
	.iter()
	{
		let uuid = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					permission
				WHERE
					id = $1;
				"#,
				uuid.as_bytes().as_ref()
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		};
		let uuid = uuid.as_bytes().as_ref();
		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, NULL);
			"#,
			uuid,
			permission
		)
		.execute(&mut *connection)
		.await?;
	}
	Ok(())
}
