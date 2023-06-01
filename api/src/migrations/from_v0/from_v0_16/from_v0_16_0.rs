use api_models::utils::Uuid;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	// feature specific migrations
	add_loki_push_permission(connection, config).await?;

	// common migrations
	reset_permission_order(connection, config).await?;

	Ok(())
}

async fn add_loki_push_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for &permission in ["workspace::region::loki_push"].iter() {
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
				&uuid
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				break uuid;
			}
		};

		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, '');
			"#,
			&uuid,
			permission
		)
		.fetch_optional(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		"workspace::infrastructure::managedUrl::list",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		"workspace::secret::list",
		"workspace::secret::create",
		"workspace::secret::edit",
		"workspace::secret::delete",
		"workspace::rbac::role::list",
		"workspace::rbac::role::create",
		"workspace::rbac::role::edit",
		"workspace::rbac::role::delete",
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
		"workspace::region::list",
		"workspace::region::info",
		"workspace::region::check_status",
		"workspace::region::add",
		"workspace::region::delete",
		"workspace::region::loki_push",
		"workspace::ci::recent_activity",
		"workspace::ci::git_provider::list",
		"workspace::ci::git_provider::connect",
		"workspace::ci::git_provider::disconnect",
		"workspace::ci::git_provider::repo::activate",
		"workspace::ci::git_provider::repo::deactivate",
		"workspace::ci::git_provider::repo::list",
		"workspace::ci::git_provider::repo::info",
		"workspace::ci::git_provider::repo::write",
		"workspace::ci::git_provider::repo::build::list",
		"workspace::ci::git_provider::repo::build::cancel",
		"workspace::ci::git_provider::repo::build::info",
		"workspace::ci::git_provider::repo::build::start",
		"workspace::ci::git_provider::repo::build::restart",
		"workspace::ci::runner::list",
		"workspace::ci::runner::create",
		"workspace::ci::runner::info",
		"workspace::ci::runner::update",
		"workspace::ci::runner::delete",
		"workspace::billing::info",
		"workspace::billing::make_payment",
		"workspace::billing::payment_method::add",
		"workspace::billing::payment_method::delete",
		"workspace::billing::payment_method::list",
		"workspace::billing::payment_method::edit",
		"workspace::billing::billing_address::add",
		"workspace::billing::billing_address::delete",
		"workspace::billing::billing_address::info",
		"workspace::billing::billing_address::edit",
		"workspace::edit",
		"workspace::delete",
	] {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = CONCAT('test::', name)
			WHERE
				name = $1;
			"#,
			permission,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				permission
			SET
				name = $1
			WHERE
				name = CONCAT('test::', $1);
			"#,
			&permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
