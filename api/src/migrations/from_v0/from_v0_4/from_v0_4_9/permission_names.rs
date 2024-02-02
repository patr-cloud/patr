use crate::{migrate_query as query, utils::Error, Database};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), Error> {
	let permission_names = [
		// Deployments
		(
			"workspace::deployment::list",
			"workspace::infrastructure::deployment::list",
		),
		(
			"workspace::deployment::create",
			"workspace::infrastructure::deployment::create",
		),
		(
			"workspace::deployment::info",
			"workspace::infrastructure::deployment::info",
		),
		(
			"workspace::deployment::delete",
			"workspace::infrastructure::deployment::delete",
		),
		(
			"workspace::deployment::edit",
			"workspace::infrastructure::deployment::edit",
		),
		// Upgrade paths
		(
			"workspace::deployment::upgradePath::list",
			"workspace::infrastructure::upgradePath::list",
		),
		(
			"workspace::deployment::upgradePath::create",
			"workspace::infrastructure::upgradePath::create",
		),
		(
			"workspace::deployment::upgradePath::info",
			"workspace::infrastructure::upgradePath::info",
		),
		(
			"workspace::deployment::upgradePath::delete",
			"workspace::infrastructure::upgradePath::delete",
		),
		(
			"workspace::deployment::upgradePath::edit",
			"workspace::infrastructure::upgradePath::edit",
		),
		// Entry points
		(
			"workspace::deployment::entryPoint::list",
			"workspace::infrastructure::entryPoint::list",
		),
		(
			"workspace::deployment::entryPoint::create",
			"workspace::infrastructure::entryPoint::create",
		),
		(
			"workspace::deployment::entryPoint::info",
			"workspace::infrastructure::entryPoint::info",
		),
		(
			"workspace::deployment::entryPoint::delete",
			"workspace::infrastructure::entryPoint::delete",
		),
		(
			"workspace::deployment::entryPoint::edit",
			"workspace::infrastructure::entryPoint::edit",
		),
		// Managed databases
		(
			"workspace::managedDatabase::create",
			"workspace::infrastructure::managedDatabase::create",
		),
		(
			"workspace::managedDatabase::list",
			"workspace::infrastructure::managedDatabase::list",
		),
		(
			"workspace::managedDatabase::delete",
			"workspace::infrastructure::managedDatabase::delete",
		),
		(
			"workspace::managedDatabase::info",
			"workspace::infrastructure::managedDatabase::info",
		),
		// Static sites
		(
			"workspace::staticSite::list",
			"workspace::infrastructure::staticSite::list",
		),
		(
			"workspace::staticSite::create",
			"workspace::infrastructure::staticSite::create",
		),
		(
			"workspace::staticSite::info",
			"workspace::infrastructure::staticSite::info",
		),
		(
			"workspace::staticSite::delete",
			"workspace::infrastructure::staticSite::delete",
		),
		(
			"workspace::staticSite::edit",
			"workspace::infrastructure::staticSite::edit",
		),
	];

	for (old_name, new_name) in permission_names {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = $1
			WHERE
				name = $2;
			"#,
			new_name,
			old_name,
		)
		.execute(&mut *connection)
		.await?;
	}
	Ok(())
}