use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

mod bytea_to_uuid;
mod docker_registry;
mod kubernetes_migration;
mod organisation_to_workspace;
mod permission_names;
mod workspace_domain;

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	organisation_to_workspace::migrate(&mut *connection, config).await?;
	bytea_to_uuid::migrate(&mut *connection, config).await?;
	permission_names::migrate(&mut *connection).await?;
	docker_registry::migrate(&mut *connection, config).await?;
	make_permission_name_unique(&mut *connection, config).await?;
	rename_static_sites_to_static_site(&mut *connection, config).await?;
	workspace_domain::migrate(&mut *connection, config).await?;
	fix_user_constraints(&mut *connection, config).await?;
	kubernetes_migration::migrate(&mut *connection, config).await?;
	reset_permission_order(&mut *connection, config).await?;
	reset_resource_types_order(&mut *connection, config).await?;

	Ok(())
}

async fn fix_user_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
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
			/* Username is a-z, 0-9, _, cannot begin or end with a . or - */
			username ~ '^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$' AND
			username NOT LIKE '%..%' AND
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
		ADD CONSTRAINT user_to_sign_up_chk_username_is_valid
		CHECK(
			/* Username is a-z, 0-9, _, cannot begin or end with a . or - */
			username ~ '^[a-z0-9_][a-z0-9_\.\-]*[a-z0-9_]$' AND
			username NOT LIKE '%..%' AND
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
			DROP CONSTRAINT
				user_to_sign_up_chk_business_domain_name_is_lower_case,
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
			ADD COLUMN otp_hash_new TEXT NOT NULL DEFAULT '',
			ADD COLUMN otp_expiry_new BIGINT NOT NULL DEFAULT 0,
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
			business_name_new = business_name,
			otp_hash_new = otp_hash,
			otp_expiry_new = otp_expiry;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
			DROP CONSTRAINT user_to_sign_up_chk_business_details_valid,
			ADD CONSTRAINT user_to_sign_up_chk_business_details_valid CHECK(
				(
					account_type = 'personal' AND
					(
						business_email_local IS NULL AND
						business_domain_name IS NULL AND
						business_domain_tld IS NULL AND
						business_name_new IS NULL
					)
				) OR
				(
					account_type = 'business' AND
					(
						business_email_local IS NOT NULL AND
						business_domain_name IS NOT NULL AND
						business_domain_tld IS NOT NULL AND
						business_name_new IS NOT NULL
					)
				)
			),
			DROP COLUMN business_name,
			DROP COLUMN otp_hash,
			DROP COLUMN otp_expiry,
			ALTER COLUMN otp_hash_new DROP DEFAULT,
			ALTER COLUMN otp_expiry_new DROP DEFAULT,
			ADD CONSTRAINT user_to_sign_up_chk_expiry_unsigned CHECK(
				otp_expiry_new >= 0
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP INDEX IF EXISTS user_to_sign_up_idx_username_otp_expiry;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP INDEX IF EXISTS user_to_sign_up_idx_otp_expiry;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_to_sign_up_idx_otp_expiry
		ON
			user_to_sign_up
		(otp_expiry_new);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_to_sign_up_idx_username_otp_expiry
		ON
			user_to_sign_up
		(username, otp_expiry_new);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN business_name_new to business_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN otp_hash_new to otp_hash;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN otp_expiry_new to otp_expiry;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn make_permission_name_unique(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE permission
		ADD CONSTRAINT permission_uq_name
		UNIQUE(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_static_sites_to_static_site(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
) -> Result<(), Error> {
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
