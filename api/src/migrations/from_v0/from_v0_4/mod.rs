use semver::Version;

use crate::{migrate_query as query, utils::settings::Settings, Database};

mod bytea_to_uuid;
mod docker_registry;
mod kubernetes_migration;
mod organisation_to_workspace;
mod permission_names;
mod workspace_domain;

/// # Description
/// The function is used to migrate the database from one version to another
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `version` - A struct containing the version to upgrade from. Panics if the
///   version is not 0.x.x, more info here: [`Version`]: Version
///
/// # Return
/// This function returns Result<(), Error> containing an empty response or
/// sqlx::error
///
/// [`Constants`]: api/src/utils/constants.rs
/// [`Transaction`]: Transaction
pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	version: Version,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	match (version.major, version.minor, version.patch) {
		(0, 4, 0) => migrate_from_v0_4_0(&mut *connection, config).await?,
		(0, 4, 1) => migrate_from_v0_4_1(&mut *connection, config).await?,
		(0, 4, 2) => migrate_from_v0_4_2(&mut *connection, config).await?,
		(0, 4, 3) => migrate_from_v0_4_3(&mut *connection, config).await?,
		(0, 4, 4) => migrate_from_v0_4_4(&mut *connection, config).await?,
		(0, 4, 5) => migrate_from_v0_4_5(&mut *connection, config).await?,
		(0, 4, 6) => migrate_from_v0_4_6(&mut *connection, config).await?,
		(0, 4, 7) => migrate_from_v0_4_7(&mut *connection, config).await?,
		(0, 4, 8) => migrate_from_v0_4_8(&mut *connection, config).await?,
		(0, 4, 9) => migrate_from_v0_4_9(&mut *connection, config).await?,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}

	Ok(())
}

/// # Description
/// The function is used to get a list of all 0.3.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec![
		"0.4.0", "0.4.1", "0.4.2", "0.4.3", "0.4.4", "0.4.5", "0.4.6", "0.4.7",
		"0.4.8", "0.4.9",
	]
}

async fn migrate_from_v0_4_0(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_4_1(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_database
		SET
			name = CONCAT('patr-deleted: ', name, '-', ENCODE(id, 'hex'))
		WHERE
			name NOT LIKE 'patr-deleted: %' AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_from_v0_4_2(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_4_3(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			domain_name = CONCAT('deleted.patr.cloud.', ENCODE(id, 'hex'), domain_name)
		WHERE
			domain_name IS NOT NULL AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			domain_name = CONCAT('deleted.patr.cloud.', ENCODE(id, 'hex'), domain_name)
		WHERE
			domain_name IS NOT NULL AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployed_domain(
			deployment_id BYTEA
				CONSTRAINT deployed_domain_uq_deployment_id UNIQUE,
			static_site_id BYTEA
				CONSTRAINT deployed_domain_uq_static_site_id UNIQUE,
			domain_name VARCHAR(255) NOT NULL
				CONSTRAINT deployed_domain_uq_domain_name UNIQUE
				CONSTRAINT deployment_chk_domain_name_is_lower_case CHECK(
					domain_name = LOWER(domain_name)
				),
			CONSTRAINT deployed_domain_uq_deployment_id_domain_name UNIQUE (deployment_id, domain_name),
			CONSTRAINT deployed_domain_uq_static_site_id_domain_name UNIQUE (static_site_id, domain_name),
			CONSTRAINT deployed_domain_chk_id_domain_is_valid CHECK(
				(
					deployment_id IS NULL AND
					static_site_id IS NOT NULL
				) OR
				(
					deployment_id IS NOT NULL AND
					static_site_id IS NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			deployed_domain (deployment_id, domain_name)
			SELECT
				deployment.id,
				deployment.domain_name
			FROM
				deployment
			WHERE 
				domain_name IS NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			deployed_domain (static_site_id, domain_name)
			SELECT
				deployment_static_sites.id,
				deployment_static_sites.domain_name
			FROM
				deployment_static_sites				
			WHERE 
				domain_name IS NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_uq_id_domain_name
		UNIQUE(id, domain_name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_fk_id_domain_name
		FOREIGN KEY(id, domain_name) REFERENCES deployed_domain(deployment_id, domain_name)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ADD CONSTRAINT deployment_static_sites_uq_id_domain_name
		UNIQUE(id, domain_name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ADD CONSTRAINT deployment_static_sites_fk_id_domain_name
		FOREIGN KEY(id, domain_name) REFERENCES deployed_domain(static_site_id, domain_name)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites 
		RENAME CONSTRAINT deployment_static_sites_pk_uq_domain_name 
		TO deployment_static_sites_uq_domain_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites 
		RENAME CONSTRAINT deployment_static_sites_pk_chk_domain_name_is_lower_case 
		TO deployment_static_sites_chk_domain_name_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployed_domain
		ADD CONSTRAINT deployed_domain_fk_deployment_id_domain_name
		FOREIGN KEY(deployment_id, domain_name) REFERENCES deployment(id, domain_name)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployed_domain
		ADD CONSTRAINT deployed_domain_fk_static_site_id_domain_name
		FOREIGN KEY(static_site_id, domain_name) REFERENCES deployment_static_sites(id, domain_name)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_from_v0_4_4(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			name = TRIM(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_chk_name_is_trimmed
		CHECK(name = TRIM(name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_database
		SET
			name = TRIM(name),
			db_name = TRIM(db_name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		ADD CONSTRAINT managed_database_chk_name_is_trimmed
		CHECK(name = TRIM(name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		ADD CONSTRAINT managed_database_chk_db_name_is_trimmed
		CHECK(db_name = TRIM(db_name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			name = TRIM(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ADD CONSTRAINT deployment_static_sites_chk_name_is_trimmed
		CHECK(name = TRIM(name));
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			domain_name = CONCAT(
				'deleted.patr.cloud.',
				ENCODE(id, 'hex'),
				'.',
				REPLACE(
					domain_name,
					CONCAT(
						'deleted.patr.cloud.',
						ENCODE(id, 'hex')
					),
					''
				)
			)
		WHERE
			domain_name NOT LIKE CONCAT(
				'deleted.patr.cloud.',
				ENCODE(id, 'hex'),
				'.%'
			) AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployed_domain
		SET
			domain_name = deployment.domain_name
		FROM
			deployment
		WHERE
			deployed_domain.deployment_id = deployment.id AND
			deployment.status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_from_v0_4_5(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_4_6(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment_static_sites
		SET
			domain_name = CONCAT(
				'deleted.patr.cloud.',
				ENCODE(id, 'hex'),
				'.',
				REPLACE(
					domain_name,
					CONCAT(
						'deleted.patr.cloud.',
						ENCODE(id, 'hex')
					),
					''
				)
			)
		WHERE
			domain_name NOT LIKE CONCAT(
				'deleted.patr.cloud.',
				ENCODE(id, 'hex'),
				'.%'
			) AND
			status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployed_domain
		SET
			domain_name = deployment_static_sites.domain_name
		FROM
			deployment_static_sites
		WHERE
			deployed_domain.static_site_id = deployment_static_sites.id AND
			deployment_static_sites.status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn migrate_from_v0_4_7(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_4_8(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_4_9(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	organisation_to_workspace::migrate(&mut *connection, config).await?;
	bytea_to_uuid::migrate(&mut *connection, config).await?;
	permission_names::migrate(&mut *connection).await?;
	docker_registry::migrate(&mut *connection, config).await?;
	fix_user_constraints(&mut *connection, config).await?;
	make_permission_name_unique(&mut *connection, config).await?;
	rename_static_sites_to_static_site(&mut *connection, config).await?;
	workspace_domain::migrate(&mut *connection, config).await?;
	kubernetes_migration::migrate(&mut *connection, config).await?;
	reset_permission_order(&mut *connection, config).await?;
	reset_resource_types_order(&mut *connection, config).await?;

	Ok(())
}

async fn fix_user_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE "user"
		DROP CONSTRAINT user_chk_username_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ADD CONSTRAINT user_chk_username_is_valid
		CHECK(
			/* Username is a-z, 0-9, cannot begin or end with a . or - */
			username ~ '^(([a-z0-9])|([a-z0-9][a-z0-9\.\-]*[a-z0-9]))$' AND
			username NOT LIKE '%..%' AND
			username NOT LIKE '%--%' AND
			username NOT LIKE '%.-%' AND
			username NOT LIKE '%-.%'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		DROP CONSTRAINT user_to_sign_up_chk_username_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		DROP CONSTRAINT user_to_sign_up_chk_username_is_trimmed;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		ADD CONSTRAINT user_to_sign_up_chk_username_is_valid
		CHECK(
			/* Username is a-z, 0-9, cannot begin or end with a . or - */
			username ~ '^(([a-z0-9])|([a-z0-9][a-z0-9\.\-]*[a-z0-9]))$' AND
			username NOT LIKE '%..%' AND
			username NOT LIKE '%--%' AND
			username NOT LIKE '%.-%' AND
			username NOT LIKE '%-.%'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
			ALTER COLUMN business_domain_name
				SET DATA TYPE TEXT,
			ADD CONSTRAINT user_to_sign_up_chk_business_domain_name_is_valid
				CHECK(
					business_domain_name ~
						'^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$'
				),
			ADD COLUMN business_domain_tld TEXT,
			ADD CONSTRAINT
				user_to_sign_up_chk_business_domain_tld_is_length_valid
					CHECK(
						LENGTH(business_domain_tld) >= 2 AND
						LENGTH(business_domain_tld) <= 6
					),
			ADD CONSTRAINT user_to_sign_up_chk_business_domain_tld_is_valid
				CHECK(
					business_domain_tld ~
						'^(([a-z0-9])|([a-z0-9][a-z0-9\-\.]*[a-z0-9]))$'
				),
			ADD CONSTRAINT user_to_sign_up_fk_business_domain_tld
				FOREIGN KEY(business_domain_tld) REFERENCES domain_tld(tld),
			ADD COLUMN business_name_new VARCHAR(100),
			DROP CONSTRAINT user_to_sign_up_chk_business_name_is_lower_case,
			ADD CONSTRAINT user_to_sign_up_chk_business_name_is_lower_case
				CHECK(business_name_new = LOWER(business_name_new)),
			ADD CONSTRAINT user_to_sign_up_chk_max_domain_name_length CHECK(
					(
						LENGTH(business_domain_name) +
						LENGTH(business_domain_tld)
					) < 255
				);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			user_to_sign_up
		SET
			business_name_new = business_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		DROP COLUMN business_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn make_permission_name_unique(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE permission
		ADD CONSTRAINT permission_uq_name
		UNIQUE(name);
		"#
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

async fn rename_static_sites_to_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment_static_sites
		RENAME TO deployment_static_site;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_site
		RENAME CONSTRAINT deployment_static_sites_chk_name_is_trimmed
		TO deployment_static_site_chk_name_is_trimmed;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_site
		RENAME CONSTRAINT deployment_static_sites_pk
		TO deployment_static_site_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_site
		RENAME CONSTRAINT deployment_static_sites_uq_name_workspace_id
		TO deployment_static_site_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_site
		RENAME CONSTRAINT deployment_static_sites_fk_id_workspace_id
		TO deployment_static_site_fk_id_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	for permission in [
		// Domain permissions
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		// Dns record permissions
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		// Deployment permissions
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		// Upgrade path permissions
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		// Managed URL permissions
		"workspace::infrastructure::managedUrl::list",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		// Managed database permissions
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		// Static site permissions
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		// Docker registry permissions
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		// Workspace permissions
		"workspace::viewRoles",
		"workspace::createRole",
		"workspace::editRole",
		"workspace::deleteRole",
		"workspace::editInfo",
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

async fn reset_resource_types_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	for resource_type in [
		"workspace",
		"domain",
		"dnsRecord",
		"dockerRepository",
		"managedDatabase",
		"deployment",
		"staticSite",
		"deploymentUpgradePath",
		"managedUrl",
	] {
		query!(
			r#"
			UPDATE
				resource_type
			SET
				name = CONCAT('test::', name)
			WHERE
				name = $1;
			"#,
			&resource_type,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				resource_type
			SET
				name = $1
			WHERE
				name = CONCAT('test::', $1);
			"#,
			&resource_type,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
