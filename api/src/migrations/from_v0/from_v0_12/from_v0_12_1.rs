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
	managed_url_redirects(connection, config).await?;
	add_tables_for_k8s_database(connection, config).await?;

	reset_permission_order(connection, config).await?;

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

async fn add_tables_for_k8s_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// add k8s_database as a resource type
	let resource_type_id = loop {
		let resource_type_id = Uuid::new_v4();

		let exists = query!(
			r#"
				SELECT
					*
				FROM
					resource_type
				WHERE
					id = $1;
				"#,
			&resource_type_id
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break resource_type_id;
		}
	};

	query!(
		r#"
			INSERT INTO
				resource_type(
					id,
					name,
					description
				)
			VALUES
				($1, 'patrDatabase', '');
			"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	// add permissions for region
	for permission in [
		"workspace::infrastructure::patrDatabase::create",
		"workspace::infrastructure::patrDatabase::list",
		"workspace::infrastructure::patrDatabase::delete",
		"workspace::infrastructure::patrDatabase::info",
	] {
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

	query!(
		r#"
		CREATE TYPE PATR_DATABASE_ENGINE AS ENUM(
			'postgres',
			'mysql'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PATR_DATABASE_PLAN AS ENUM(
			'db_1r_1c_10v',
			'db_2r_2c_25v'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PATR_DATABASE_STATUS AS ENUM(
			'creating',
			'running',
			'errored',
			'deleted'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE patr_database(
			id 				UUID					NOT NULL,
			name 			CITEXT 					NOT NULL,
			workspace_id 	UUID 					NOT NULL,
			region 			UUID 					NOT NULL,
			db_name 		VARCHAR(255) 			NOT NULL,
			engine 			PATR_DATABASE_ENGINE 	NOT NULL,
			version 		TEXT 					NOT NULL,
			database_plan 	PATR_DATABASE_PLAN 		NOT NULL,
			status 			PATR_DATABASE_STATUS 	NOT NULL DEFAULT 'creating',
			host 			TEXT 					NOT NULL,
			port 			INTEGER 				NOT NULL,
			username 		TEXT 					NOT NULL,
			password 		TEXT 					NOT NULL,
			deleted 		TIMESTAMPTZ,

			CONSTRAINT patr_database_pk
				PRIMARY KEY (id),
		
			CONSTRAINT patr_database_chk_name_is_trimmed
				CHECK(name = TRIM(name)),
			CONSTRAINT patr_database_chk_db_name_is_trimmed
				CHECK(db_name = TRIM(db_name)),
		
			CONSTRAINT patr_database_fk_region
				FOREIGN KEY (region) REFERENCES deployment_region(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			patr_database_uq_workspace_id_name
		ON
			patr_database(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE patr_database
			ADD CONSTRAINT patr_database_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS patr_database_payment_history(
			workspace_id UUID NOT NULL,
			database_id UUID NOT NULL,
			db_plan PATR_DATABASE_PLAN NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			deletion_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE patr_database_payment_history
		ADD CONSTRAINT patr_database_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

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
