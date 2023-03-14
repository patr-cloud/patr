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
	managed_url_redirects(&mut *connection, config).await?;
	domain_last_unverified_nullable(connection, config).await?;
	migrate_non_unverified_domains(connection, config).await?;
	Ok(())
}

pub(super) async fn managed_url_redirects(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE managed_url
			ADD COLUMN permanent_redirect BOOLEAN,
			ADD COLUMN http_only BOOLEAN;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_url
		SET
			http_only = FALSE,
			permanent_redirect = FALSE
		WHERE
			url_type = 'redirect';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_url
		SET
			http_only = FALSE
		WHERE
			url_type = 'proxy_url';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
		DROP CONSTRAINT managed_url_chk_values_null_or_not_null;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
		ADD CONSTRAINT managed_url_chk_values_null_or_not_null CHECK(
			(
				url_type = 'proxy_to_deployment' AND
				deployment_id IS NOT NULL AND
				port IS NOT NULL AND
				static_site_id IS NULL AND
				url IS NULL AND
				permanent_redirect IS NULL AND
				http_only IS NULL
			) OR (
				url_type = 'proxy_to_static_site' AND
				deployment_id IS NULL AND
				port IS NULL AND
				static_site_id IS NOT NULL AND
				url IS NULL AND
				permanent_redirect IS NULL AND
				http_only IS NULL
			) OR (
				url_type = 'proxy_url' AND
				deployment_id IS NULL AND
				port IS NULL AND
				static_site_id IS NULL AND
				url IS NOT NULL AND
				permanent_redirect IS NULL AND
				http_only IS NOT NULL
			) OR (
				url_type = 'redirect' AND
				deployment_id IS NULL AND
				port IS NULL AND
				static_site_id IS NULL AND
				url IS NOT NULL AND
				permanent_redirect IS NOT NULL AND
				http_only IS NOT NULL
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn domain_last_unverified_nullable(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace_domain
		ALTER COLUMN last_unverified
		DROP NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

pub async fn migrate_non_unverified_domains(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		UPDATE
			workspace_domain
		SET
			last_unverified = NULL
		FROM 
			resource
		WHERE
			resource.id = workspace_domain.id
		AND
			workspace_domain.last_unverified = resource.created;
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
<<<<<<< HEAD
=======

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
		"workspace::infrastructure::patrDatabase::create",
		"workspace::infrastructure::patrDatabase::list",
		"workspace::infrastructure::patrDatabase::delete",
		"workspace::infrastructure::patrDatabase::info",
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
		"workspace::region::add",
		"workspace::ci::git_provider::connect",
		"workspace::ci::git_provider::disconnect",
		"workspace::ci::git_provider::repo::activate",
		"workspace::ci::git_provider::repo::deactivate",
		"workspace::ci::git_provider::repo::list",
		"workspace::ci::git_provider::repo::build::view",
		"workspace::ci::git_provider::repo::build::restart",
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
>>>>>>> 00462a6e (Added mongo db support with scaling)
