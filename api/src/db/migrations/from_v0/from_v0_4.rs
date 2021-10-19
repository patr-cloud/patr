use semver::Version;
use sqlx::{postgres::PgRow, Row};

use crate::{migrate_query as query, Database};

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
) -> Result<(), sqlx::Error> {
	match (version.major, version.minor, version.patch) {
		(0, 4, 0) => migrate_from_v0_4_0(&mut *connection).await?,
		(0, 4, 1) => migrate_from_v0_4_1(&mut *connection).await?,
		(0, 4, 2) => migrate_from_v0_4_2(&mut *connection).await?,
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
	vec!["0.4.0", "0.4.1", "0.4.2"]
}

async fn migrate_from_v0_4_0(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_4_1(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_4_2(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	rename_organisation_account_type_to_business(&mut *connection).await?;
	rename_organisation_email_to_business_email(&mut *connection).await?;
	rename_user_to_sign_up_columns(&mut *connection).await?;
	rename_organisation_to_workspace(&mut *connection).await?;
	rename_organisation_domain_to_workspace_domain(&mut *connection).await?;
	rename_docker_registry_repository_columns(&mut *connection).await?;
	rename_deployment_columns(&mut *connection).await?;
	rename_managed_database_columns(&mut *connection).await?;
	rename_static_sites_columns(&mut *connection).await?;
	rename_resource_columns(&mut *connection).await?;
	rename_organisation_user_to_workspace_user(&mut *connection).await?;
	remove_application_tables(&mut *connection).await?;
	remove_application_permissions(&mut *connection).await?;
	remove_application_resource_type(&mut *connection).await?;
	remove_drive_tables(&mut *connection).await?;
	remove_portus_tables(&mut *connection).await?;
	remove_portus_permissions(&mut *connection).await?;
	remove_portus_resource_type(&mut *connection).await?;
	rename_all_permissions(&mut *connection).await?;
	rename_organisation_resource_type_to_workspace(&mut *connection).await?;
	rename_organisation_resource_names_to_workspace(&mut *connection).await?;
	rename_personal_workspace_names(&mut *connection).await?;

	Ok(())
}

async fn rename_organisation_account_type_to_business(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TYPE RESOURCE_OWNER_TYPE
		RENAME VALUE 'organisation'
		TO 'business';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_organisation_email_to_business_email(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE organisation_email
		RENAME TO business_email;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE business_email
		RENAME CONSTRAINT organisation_email_fk_user_id
		TO business_email_fk_user_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE business_email
		RENAME CONSTRAINT organisation_email_chk_local_is_lower_case
		TO business_email_chk_local_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE business_email
		RENAME CONSTRAINT organisation_email_fk_domain_id
		TO business_email_fk_domain_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE business_email
		RENAME CONSTRAINT organisation_email_pk
		TO business_email_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX organisation_email_idx_user_id
		RENAME TO business_email_idx_user_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_user_to_sign_up_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN org_email_local
		TO business_email_local;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_chk_org_email_is_lower_case
		TO user_to_sign_up_chk_business_email_local_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN org_domain_name
		TO business_domain_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_chk_org_domain_name_is_lower_case
		TO user_to_sign_up_chk_business_domain_name_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN organisation_name
		TO business_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_chk_org_name_is_lower_case
		TO user_to_sign_up_chk_business_name_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_chk_org_details_valid
		TO user_to_sign_up_chk_business_details_valid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_organisation_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE organisation
		RENAME TO workspace;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		RENAME CONSTRAINT organisation_pk
		TO workspace_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		RENAME CONSTRAINT organisation_uq_name
		TO workspace_uq_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN name
		SET DATA TYPE CITEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		RENAME CONSTRAINT organisation_super_admin_id_fk_user_id
		TO workspace_super_admin_id_fk_user_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		RENAME CONSTRAINT organisation_fk_id
		TO workspace_fk_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX organisation_idx_super_admin_id
		RENAME TO workspace_idx_super_admin_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX organisation_idx_active
		RENAME TO workspace_idx_active;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_organisation_domain_to_workspace_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE organisation_domain
		RENAME TO workspace_domain;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
		RENAME CONSTRAINT organisation_domain_pk
		TO workspace_domain_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
		DROP CONSTRAINT organisation_domain_chk_dmn_typ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
		ADD CONSTRAINT workspace_domain_chk_dmn_typ
		CHECK(domain_type = 'business');
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
		RENAME CONSTRAINT organisation_domain_fk_id_domain_type
		TO workspace_domain_fk_id_domain_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
		RENAME CONSTRAINT organisation_domain_fk_id
		TO workspace_domain_fk_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX organisation_domain_idx_is_verified
		RENAME TO workspace_domain_idx_is_verified;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_docker_registry_repository_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE docker_registry_repository
		RENAME COLUMN organisation_id
		TO workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ALTER COLUMN name
		SET DATA TYPE CITEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		RENAME CONSTRAINT docker_registry_repository_fk_id
		TO docker_registry_repository_fk_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		RENAME CONSTRAINT docker_registry_repository_uq_organisation_id_name
		TO docker_registry_repository_uq_workspace_id_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		RENAME CONSTRAINT docker_registry_repository_fk_id_organisation_id
		TO docker_registry_repository_fk_id_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_deployment_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment
		RENAME COLUMN organisation_id
		TO workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		RENAME CONSTRAINT deployment_uq_name_organisation_id
		TO deployment_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ALTER COLUMN name
		SET DATA TYPE CITEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		RENAME CONSTRAINT deployment_fk_id_organisation_id
		TO deployment_fk_id_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_managed_database_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE managed_database
		RENAME COLUMN organisation_id
		TO workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		RENAME CONSTRAINT managed_database_uq_name_organisation_id
		TO managed_database_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		ALTER COLUMN name
		SET DATA TYPE CITEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		RENAME CONSTRAINT managed_database_repository_fk_id_organisation_id
		TO managed_database_fk_id_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_static_sites_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment_static_sites
		RENAME COLUMN organisation_id
		TO workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		RENAME CONSTRAINT deployment_static_sites_uq_name_organisation_id
		TO deployment_static_sites_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ALTER COLUMN name
		SET DATA TYPE CITEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		RENAME CONSTRAINT deployment_static_sites_fk_id_organisation_id
		TO deployment_static_sites_fk_id_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_resource_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE resource
		RENAME CONSTRAINT organisation_created_chk_unsigned
		TO resource_created_chk_unsigned;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_organisation_user_to_workspace_user(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE organisation_user
		RENAME TO workspace_user;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		RENAME CONSTRAINT organisation_user_fk_user_id
		TO workspace_user_fk_user_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		RENAME COLUMN organisation_id
		TO workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		RENAME CONSTRAINT organisation_user_fk_organisation_id
		TO workspace_user_fk_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		RENAME CONSTRAINT organisation_user_fk_role_id
		TO workspace_user_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		RENAME CONSTRAINT organisation_user_pk
		TO workspace_user_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX organisation_user_idx_user_id
		RENAME TO workspace_user_idx_user_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX organisation_user_idx_user_id_organisation_id
		RENAME TO workspace_user_idx_user_id_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn remove_application_tables(
	connection: &mut sqlx::PgConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DROP TABLE application_version_platform;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE application_version;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE application;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn remove_application_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	for permission in [
		"organisation::application::list",
		"organisation::application::add",
		"organisation::application::viewDetails",
		"organisation::application::delete",
		"organisation::application::listVersions",
	]
	.iter()
	{
		let permission_row = query!(
			r#"
			SELECT
				*
			FROM
				permission
			WHERE
				name = $1;
			"#,
			permission,
		)
		.map(|row| row.get::<Vec<u8>, _>("id"))
		.fetch_optional(&mut *connection)
		.await?;

		let permission_id = if let Some(permission) = permission_row {
			permission
		} else {
			continue;
		};

		query!(
			r#"
			DELETE FROM
				role_permissions_resource
			WHERE
				permission_id = $1;
			"#,
			&permission_id
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			DELETE FROM
				role_permissions_resource_type
			WHERE
				permission_id = $1;
			"#,
			&permission_id
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			DELETE FROM
				permission
			WHERE
				name = $1;
			"#,
			permission
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn remove_application_resource_type(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	let resource_type_row = query!(
		r#"
		SELECT
			*
		FROM
			resource_type
		WHERE
			name = 'application';
		"#
	)
	.map(|row: PgRow| row.get::<Vec<u8>, _>("id"))
	.fetch_optional(&mut *connection)
	.await?;

	let resource_type_id = if let Some(resource_type_id) = resource_type_row {
		resource_type_id
	} else {
		return Ok(());
	};

	query!(
		r#"
		DELETE FROM
			resource
		WHERE
			resource_type_id = $1;
		"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			resource_type
		WHERE
			id = $1;
		"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn remove_drive_tables(
	connection: &mut sqlx::PgConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DROP TABLE file;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn remove_portus_tables(
	connection: &mut sqlx::PgConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DROP INDEX portus_tunnel_idx_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE portus_tunnel;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn remove_portus_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	for permission in [
		"organisation::portus::add",
		"organisation::portus::view",
		"organisation::portus::list",
		"organisation::portus::delete",
	]
	.iter()
	{
		let permission_row = query!(
			r#"
			SELECT
				*
			FROM
				permission
			WHERE
				name = $1;
			"#,
			permission,
		)
		.map(|row| row.get::<Vec<u8>, _>("id"))
		.fetch_optional(&mut *connection)
		.await?;

		let permission_id = if let Some(permission) = permission_row {
			permission
		} else {
			continue;
		};

		query!(
			r#"
			DELETE FROM
				role_permissions_resource
			WHERE
				permission_id = $1;
			"#,
			&permission_id
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			DELETE FROM
				role_permissions_resource_type
			WHERE
				permission_id = $1;
			"#,
			&permission_id
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			DELETE FROM
				permission
			WHERE
				name = $1;
			"#,
			permission
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn remove_portus_resource_type(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	let resource_type_row = query!(
		r#"
		SELECT
			*
		FROM
			resource_type
		WHERE
			name = 'application';
		"#
	)
	.map(|row: PgRow| row.get::<Vec<u8>, _>("id"))
	.fetch_optional(&mut *connection)
	.await?;

	let resource_type_id = if let Some(resource_type_id) = resource_type_row {
		resource_type_id
	} else {
		return Ok(());
	};

	query!(
		r#"
		DELETE FROM
			resource
		WHERE
			resource_type_id = $1;
		"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			resource_type
		WHERE
			id = $1;
		"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_all_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			permission
		SET
			name = REPLACE(name, 'organisation::', 'workspace::');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_organisation_resource_type_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			resource_type
		SET
			name = 'workspace'
		WHERE
			name = 'organisation';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_organisation_resource_names_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			resource
		SET
			name = REPLACE(name, 'Organisation: ', 'Workspace: ');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rename_personal_workspace_names(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			workspace
		SET
			name = (
				SELECT
					CONCAT('personal-workspace-', ENCODE("user".id, 'hex'))
				FROM
					"user"
				WHERE
					username = REPLACE(name, 'personal-organisation-', '')
			)
		WHERE
			name LIKE 'personal-organisation-%';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
