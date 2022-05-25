use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	remove_list_permission(&mut *connection).await?;
	Ok(())
}

async fn remove_list_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), Error> {
	for &permission in [
		"workspace::infrastructure::deployment::list",
		"workspace::dockerRegistry::list",
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::managedUrl::info",
	]
	.iter()
	{
		query!(
			r#"
			DELETE
				FROM permission
			WHERE
				name = $1;
			"#,
			permission,
		)
		.execute(&mut *connection)
		.await?;
	}
	Ok(())
}
