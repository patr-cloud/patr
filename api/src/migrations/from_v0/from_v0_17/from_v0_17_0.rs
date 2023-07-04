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
	add_reconfigure_region_permission(connection, config).await?;
	reset_permission_order(connection, config).await?;

	Ok(())
}

pub async fn add_reconfigure_region_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let permissions = ["workspace::region::reconfigure"];

	for permission in permissions {
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
		"workspace::domain::add",
		"workspace::domain::info",
		"workspace::domain::verify",
		"workspace::domain::delete",
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		"workspace::infrastructure::managedUrl::info",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		"workspace::containerRegistry::create",
		"workspace::containerRegistry::delete",
		"workspace::containerRegistry::info",
		"workspace::containerRegistry::push",
		"workspace::containerRegistry::pull",
		"workspace::secret::info",
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
		"workspace::region::info",
		"workspace::region::checkStatus",
		"workspace::region::add",
		"workspace::region::delete",
		"workspace::region::reconfigure",
		"workspace::region::pushLogs",
		"workspace::region::pushMetrics",
		"workspace::ci::recentActivity",
		"workspace::ci::gitProvider::list",
		"workspace::ci::gitProvider::connect",
		"workspace::ci::gitProvider::disconnect",
		"workspace::ci::gitProvider::repo::activate",
		"workspace::ci::gitProvider::repo::deactivate",
		"workspace::ci::gitProvider::repo::sync",
		"workspace::ci::gitProvider::repo::info",
		"workspace::ci::gitProvider::repo::write",
		"workspace::ci::gitProvider::repo::build::list",
		"workspace::ci::gitProvider::repo::build::cancel",
		"workspace::ci::gitProvider::repo::build::info",
		"workspace::ci::gitProvider::repo::build::start",
		"workspace::ci::gitProvider::repo::build::restart",
		"workspace::ci::runner::create",
		"workspace::ci::runner::info",
		"workspace::ci::runner::update",
		"workspace::ci::runner::delete",
		"workspace::billing::info",
		"workspace::billing::makePayment",
		"workspace::billing::paymentMethod::add",
		"workspace::billing::paymentMethod::delete",
		"workspace::billing::paymentMethod::list",
		"workspace::billing::paymentMethod::edit",
		"workspace::billing::billingAddress::add",
		"workspace::billing::billingAddress::delete",
		"workspace::billing::billingAddress::info",
		"workspace::billing::billingAddress::edit",
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
			permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
