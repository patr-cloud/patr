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
	add_tables_for_k8s_database(connection, config).await?;
	reset_permission_order(connection, config).await?;
	reset_resource_order(connection, config).await?;
	add_is_frozen_column_to_workspace(connection, config).await?;
	add_survey_table(connection, config).await?;

	Ok(())
}

async fn add_is_frozen_column_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN is_frozen BOOLEAN NOT NULL DEFAULT FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN is_frozen DROP DEFAULT;
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
			'mysql',
			'mongo',
			'redis'
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
				FOREIGN KEY (region) REFERENCES region(id)
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
		"workspace::region::info",
		"workspace::region::check_status",
		"workspace::region::add",
		"workspace::region::delete",
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

async fn reset_resource_order(
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
		"secret",
		"staticSiteUpload",
		"deploymentRegion",
		"deploymentVolume",
		"patrDatabase",
		"ciRepo",
		"ciRunner",
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
			resource_type,
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

async fn add_survey_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE SURVEY(
			user_id UUID NOT NULL
				CONSTRAINT survey_fk_user_id REFERENCES "user"(id),
			version TEXT,
			response JSON NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
