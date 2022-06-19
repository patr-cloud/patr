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
	rbac_related_migrations(&mut *connection, config).await?;

	Ok(())
}

async fn rbac_related_migrations(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	// block permission changes
	rename_role_permissions_resource_tables(&mut *connection, config).await?;
	create_role_block_permissions_resource_table(&mut *connection, config)
		.await?;

	// permission to resource mapping changes
	add_validation_for_permissions_on_resource(&mut *connection, config)
		.await?;

	// project related changes
	add_resource_type_and_permissions_for_projects(&mut *connection, config)
		.await?;
	create_project_tables(&mut *connection, config).await?;

	Ok(())
}

async fn rename_role_permissions_resource_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE
			role_permissions_resource
		RENAME TO
			role_allow_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_permissions_resource_type
		RENAME TO
			role_allow_permissions_resource_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_role_id
		TO
			role_allow_permissions_resource_type_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_permission_id
		TO
			role_allow_permissions_resource_type_fk_permission_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_resource_type_id
		TO
			role_allow_permissions_resource_type_fk_resource_type_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_pk
		TO
			role_allow_permissions_resource_type_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_role_id
		TO
			role_allow_permissions_resource_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_permission_id
		TO
			role_allow_permissions_resource_fk_permission_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_resource_id
		TO
			role_allow_permissions_resource_fk_resource_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_pk
		TO
			role_allow_permissions_resource_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_type_idx_role_id
		RENAME TO
			role_allow_permissions_resource_type_idx_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_type_idx_role_id_resource_type_id
		RENAME TO
			role_allow_permissions_resource_type_idx_roleid_resourcetypeid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_idx_role_id
		RENAME TO
			role_allow_permissions_resource_idx_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_idx_role_id_resource_id
		RENAME TO
			role_allow_permissions_resource_idx_role_id_resource_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn create_role_block_permissions_resource_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE role_block_permissions_resource(
			role_id UUID
				CONSTRAINT role_block_permissions_resource_fk_role_id
					REFERENCES role(id),
			permission_id UUID
				CONSTRAINT role_block_permissions_resource_fk_permission_id
					REFERENCES permission(id),
			resource_id UUID
				CONSTRAINT role_block_permissions_resource_fk_resource_id
					REFERENCES resource(id),
			CONSTRAINT role_block_permissions_resource_pk
				PRIMARY KEY(role_id, permission_id, resource_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_block_permissions_resource_idx_role_id
		ON
			role_block_permissions_resource(role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_block_permissions_resource_idx_role_id_resource_id
		ON
			role_block_permissions_resource(role_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_validation_for_permissions_on_resource(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// create permission to resource mapping function
	query!(
		r#"
		CREATE OR REPLACE FUNCTION validate_permission_to_resource_mapping(
			permission_name TEXT,
			resource_type_name TEXT
		) RETURNS BOOLEAN AS $$
		SELECT CASE
			resource_type_name
			WHEN 'workspace' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::list',
							'workspace::domain::add',

							'workspace::infrastructure::deployment::list',
							'workspace::infrastructure::deployment::create',

							'workspace::infrastructure::managedUrl::list',
							'workspace::infrastructure::managedUrl::create',

							'workspace::infrastructure::managedDatabase::create',
							'workspace::infrastructure::managedDatabase::list',

							'workspace::dockerRegistry::create',
							'workspace::dockerRegistry::list',

							'workspace::secret::list',
							'workspace::secret::create',

							'workspace::infrastructure::staticSite::list',
							'workspace::infrastructure::staticSite::create',

							'workspace::infrastructure::upgradePath::list',
							'workspace::infrastructure::upgradePath::create',

							'workspace::rbac::role::list',
							'workspace::rbac::role::create',
							'workspace::rbac::role::edit',
							'workspace::rbac::role::delete',

							'workspace::rbac::user::list',
							'workspace::rbac::user::add',
							'workspace::rbac::user::remove',
							'workspace::rbac::user::updateRoles',

							'workspace::edit',
							'workspace::delete',

							'workspace::ci::github::connect',
							'workspace::ci::github::activate',
							'workspace::ci::github::deactivate',
							'workspace::ci::github::viewBuilds',
							'workspace::ci::github::restartBuilds',
							'workspace::ci::github::disconnect'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'domain' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::viewDetails',
							'workspace::domain::verify',
							'workspace::domain::delete',

							'workspace::domain::dnsRecord::list',
							'workspace::domain::dnsRecord::add'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'dnsRecord' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::dnsRecord::edit',
							'workspace::domain::dnsRecord::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'dockerRepository' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::dockerRegistry::delete',
							'workspace::dockerRegistry::info',
							'workspace::dockerRegistry::push',
							'workspace::dockerRegistry::pull'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'managedDatabase' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::managedDatabase::delete',
							'workspace::infrastructure::managedDatabase::info'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'deployment' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::deployment::info',
							'workspace::infrastructure::deployment::delete',
							'workspace::infrastructure::deployment::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'staticSite' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::staticSite::info',
							'workspace::infrastructure::staticSite::delete',
							'workspace::infrastructure::staticSite::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'deploymentUpgradePath' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::upgradePath::info',
							'workspace::infrastructure::upgradePath::delete',
							'workspace::infrastructure::upgradePath::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'managedUrl' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::managedUrl::edit',
							'workspace::infrastructure::managedUrl::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'secret' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::secret::edit',
							'workspace::secret::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			ELSE FALSE
		END;
		$$ LANGUAGE SQL IMMUTABLE STRICT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// create triggers for role_allow_permissions_resource
	query!(
		r#"
		CREATE OR REPLACE FUNCTION
			role_allow_permissions_resource_check()
		RETURNS TRIGGER AS $$
		DECLARE
			permission_name TEXT;
			resource_type_name TEXT;
		BEGIN
			SELECT
				permission.name INTO permission_name
			FROM
				permission
			WHERE
				permission.id = NEW.permission_id;

			SELECT
				resource_type.name INTO resource_type_name
			FROM
				resource_type
			JOIN
				resource ON resource.resource_type_id = resource_type.id
			WHERE
				resource.id = NEW.resource_id;

			IF validate_permission_to_resource_mapping(permission_name, resource_type_name) THEN
				RETURN NEW;
			END IF;

			RAISE EXCEPTION 'Invalid permission is provided for resource';
		END
		$$ LANGUAGE PLPGSQL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TRIGGER IF EXISTS
			role_allow_permissions_resource_check_trigger
		ON
			role_allow_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TRIGGER
			role_allow_permissions_resource_check_trigger
		BEFORE INSERT OR UPDATE ON
			role_allow_permissions_resource
		FOR EACH ROW EXECUTE FUNCTION
			role_allow_permissions_resource_check();
		"#
	)
	.execute(&mut *connection)
	.await?;

	// create triggers for role_block_permissions_resource
	query!(
		r#"
		CREATE OR REPLACE FUNCTION
			role_block_permissions_resource_check()
		RETURNS TRIGGER AS $$
		DECLARE
			permission_name TEXT;
			resource_type_name TEXT;
		BEGIN
			SELECT
				permission.name INTO permission_name
			FROM
				permission
			WHERE
				permission.id = NEW.permission_id;

			SELECT
				resource_type.name INTO resource_type_name
			FROM
				resource_type
			JOIN
				resource ON resource.resource_type_id = resource_type.id
			WHERE
				resource.id = NEW.resource_id;

			IF validate_permission_to_resource_mapping(permission_name, resource_type_name) THEN
				RETURN NEW;
			END IF;

			RAISE EXCEPTION 'Invalid permission is provided for resource';
		END
		$$ LANGUAGE PLPGSQL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TRIGGER IF EXISTS
			role_block_permissions_resource_check_trigger
		ON
			role_block_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TRIGGER
			role_block_permissions_resource_check_trigger
		BEFORE INSERT OR UPDATE ON
			role_block_permissions_resource
		FOR EACH ROW EXECUTE FUNCTION
			role_block_permissions_resource_check();
		"#
	)
	.execute(&mut *connection)
	.await?;

	// create triggers for role_allow_permissions_resource_type
	query!(
		r#"
		CREATE OR REPLACE FUNCTION
			role_allow_permissions_resource_type_check()
		RETURNS TRIGGER AS $$
		DECLARE
			permission_name TEXT;
			resource_type_name TEXT;
		BEGIN
			SELECT
				permission.name INTO permission_name
			FROM
				permission
			WHERE
				permission.id = NEW.permission_id;

			SELECT
				resource_type.name INTO resource_type_name
			FROM
				resource_type
			WHERE
				resource_type.id = NEW.resource_type_id;

			IF validate_permission_to_resource_mapping(permission_name, resource_type_name) THEN
				RETURN NEW;
			END IF;

			RAISE EXCEPTION 'Invalid permission is provided for resource';
		END
		$$ LANGUAGE PLPGSQL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TRIGGER IF EXISTS
			role_allow_permissions_resource_type_check_trigger
		ON
			role_allow_permissions_resource_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TRIGGER
			role_allow_permissions_resource_type_check_trigger
		BEFORE INSERT OR UPDATE ON
			role_allow_permissions_resource_type
		FOR EACH ROW EXECUTE FUNCTION
			role_allow_permissions_resource_type_check();
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_resource_type_and_permissions_for_projects(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// add project as a resource type
	const PROJECT_RESOURCE_TYPE_NAME: &str = "project";
	let project_resource_type_id = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
				SELECT
					*
				FROM
					resource_type
				WHERE
					id = $1;
				"#,
			&uuid
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			// That particular resource ID doesn't exist. Use it
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO
			resource_type
		VALUES
			($1, $2, '');
		"#,
		&project_resource_type_id,
		PROJECT_RESOURCE_TYPE_NAME,
	)
	.execute(&mut *connection)
	.await?;

	// add permissions for project resource type
	for &permission in [
		"workspace::project::list",
		"workspace::project::create",
		"workspace::project::info",
		"workspace::project::delete",
		"workspace::project::edit",
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
				&uuid
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		};
		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, $3);
			"#,
			&uuid,
			permission,
			"",
		)
		.execute(&mut *connection)
		.await?;
	}

	// update the permission to role mapping for project resource type
	query!(
		r#"
		CREATE OR REPLACE FUNCTION validate_permission_to_resource_mapping(
			permission_name TEXT,
			resource_type_name TEXT
		) RETURNS BOOLEAN AS $$
		SELECT CASE
			resource_type_name
			WHEN 'workspace' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::list',
							'workspace::domain::add',

							'workspace::infrastructure::deployment::list',
							'workspace::infrastructure::deployment::create',

							'workspace::infrastructure::managedUrl::list',
							'workspace::infrastructure::managedUrl::create',

							'workspace::infrastructure::managedDatabase::create',
							'workspace::infrastructure::managedDatabase::list',

							'workspace::dockerRegistry::create',
							'workspace::dockerRegistry::list',

							'workspace::secret::list',
							'workspace::secret::create',

							'workspace::infrastructure::staticSite::list',
							'workspace::infrastructure::staticSite::create',

							'workspace::infrastructure::upgradePath::list',
							'workspace::infrastructure::upgradePath::create',

							'workspace::rbac::role::list',
							'workspace::rbac::role::create',
							'workspace::rbac::role::edit',
							'workspace::rbac::role::delete',

							'workspace::rbac::user::list',
							'workspace::rbac::user::add',
							'workspace::rbac::user::remove',
							'workspace::rbac::user::updateRoles',

							'workspace::edit',
							'workspace::delete',

							'workspace::ci::github::connect',
							'workspace::ci::github::activate',
							'workspace::ci::github::deactivate',
							'workspace::ci::github::viewBuilds',
							'workspace::ci::github::restartBuilds',
							'workspace::ci::github::disconnect',

							'workspace::project::list',
							'workspace::project::create'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'domain' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::viewDetails',
							'workspace::domain::verify',
							'workspace::domain::delete',

							'workspace::domain::dnsRecord::list',
							'workspace::domain::dnsRecord::add'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'dnsRecord' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::dnsRecord::edit',
							'workspace::domain::dnsRecord::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'dockerRepository' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::dockerRegistry::delete',
							'workspace::dockerRegistry::info',
							'workspace::dockerRegistry::push',
							'workspace::dockerRegistry::pull'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'managedDatabase' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::managedDatabase::delete',
							'workspace::infrastructure::managedDatabase::info'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'deployment' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::deployment::info',
							'workspace::infrastructure::deployment::delete',
							'workspace::infrastructure::deployment::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'staticSite' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::staticSite::info',
							'workspace::infrastructure::staticSite::delete',
							'workspace::infrastructure::staticSite::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'deploymentUpgradePath' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::upgradePath::info',
							'workspace::infrastructure::upgradePath::delete',
							'workspace::infrastructure::upgradePath::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'managedUrl' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::managedUrl::edit',
							'workspace::infrastructure::managedUrl::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'secret' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::secret::edit',
							'workspace::secret::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'project' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::project::info',
							'workspace::project::delete',
							'workspace::project::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			ELSE FALSE
		END;
		$$ LANGUAGE SQL IMMUTABLE STRICT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn create_project_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE project (
			id UUID
				CONSTRAINT project_pk PRIMARY KEY,
			workspace_id UUID NOT NULL
				CONSTRAINT project_fk_workspace_id REFERENCES workspace(id),
			name CITEXT NOT NULL,
			description VARCHAR(500) NOT NULL,
			deleted BOOLEAN NOT NULL DEFAULT FALSE,

			CONSTRAINT project_fk_id_workspace_id
				FOREIGN KEY (id, workspace_id)
					REFERENCES resource(id, owner_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			project_uq_idx_name_workspace_id
		ON
			project(name, workspace_id)
		WHERE
			deleted IS FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
