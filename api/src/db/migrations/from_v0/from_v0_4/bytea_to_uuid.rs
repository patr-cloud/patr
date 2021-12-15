use sqlx::Row;

use crate::{migrate_query as query, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	let constraint_list =
		mark_all_non_deferrable_constraints_deferrable(&mut *connection)
			.await?;
	alter_user_tables(&mut *connection).await?;
	alter_workspace_tables(&mut *connection).await?;
	alter_rbac_tables(&mut *connection).await?;
	mark_all_non_deferrable_constraints_non_deferrable(
		&mut *connection,
		&constraint_list,
	)
	.await?;

	Ok(())
}

async fn mark_all_non_deferrable_constraints_deferrable(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<(String, String)>, sqlx::Error> {
	let constraints = query!(
		r#"
		SELECT
			pg_constraint.conname,
			pg_class.relname
		FROM
			pg_catalog.pg_constraint
		INNER JOIN
			pg_catalog.pg_class
		ON
			pg_class.oid = pg_constraint.conrelid
		INNER JOIN
			pg_catalog.pg_namespace
		ON
			pg_namespace.oid = connamespace
		WHERE
			pg_namespace.nspname = 'public' AND
			pg_constraint.condeferrable = 'f' AND
			pg_constraint.conname LIKE CONCAT(pg_class.relname, '_fk_%');
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.get("relname"), row.get("conname")))
	.collect::<Vec<_>>();

	for (table_name, constraint_name) in &constraints {
		sqlx::query(&format!(
			r#"
			ALTER TABLE {}
			ALTER CONSTRAINT {}
			DEFERRABLE INITIALLY DEFERRED;
			"#,
			table_name, constraint_name,
		))
		.execute(&mut *connection)
		.await?;
	}

	Ok(constraints)
}

async fn alter_rbac_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE permission
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE resource
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE resource
		ALTER COLUMN resource_type_id TYPE UUID
		USING CAST(ENCODE(resource_type_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE resource
		ALTER COLUMN owner_id TYPE UUID
		USING CAST(ENCODE(owner_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role
		ALTER COLUMN owner_id TYPE UUID
		USING CAST(ENCODE(owner_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE permission
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		ALTER COLUMN user_id TYPE UUID
		USING CAST(ENCODE(user_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		ALTER COLUMN workspace_id TYPE UUID
		USING CAST(ENCODE(workspace_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		ALTER COLUMN role_id TYPE UUID
		USING CAST(ENCODE(role_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_permissions_resource_type
		ALTER COLUMN role_id TYPE UUID
		USING CAST(ENCODE(role_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_permissions_resource_type
		ALTER COLUMN permission_id TYPE UUID
		USING CAST(ENCODE(permission_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_permissions_resource_type
		ALTER COLUMN resource_type_id TYPE UUID
		USING CAST(ENCODE(resource_type_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_permissions_resource
		ALTER COLUMN role_id TYPE UUID
		USING CAST(ENCODE(role_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_permissions_resource
		ALTER COLUMN permission_id TYPE UUID
		USING CAST(ENCODE(permission_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_permissions_resource
		ALTER COLUMN resource_id TYPE UUID
		USING CAST(ENCODE(resource_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn alter_user_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE "user"
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ALTER COLUMN backup_email_domain_id TYPE UUID
		USING CAST(ENCODE(backup_email_domain_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
		ALTER COLUMN login_id TYPE UUID
		USING CAST(ENCODE(login_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
		ALTER COLUMN user_id TYPE UUID
		USING CAST(ENCODE(user_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE password_reset_request
		ALTER COLUMN user_id TYPE UUID
		USING CAST(ENCODE(user_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE personal_email
		ALTER COLUMN user_id TYPE UUID
		USING CAST(ENCODE(user_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE personal_email
		ALTER COLUMN domain_id TYPE UUID
		USING CAST(ENCODE(domain_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE business_email
		ALTER COLUMN user_id TYPE UUID
		USING CAST(ENCODE(user_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE business_email
		ALTER COLUMN domain_id TYPE UUID
		USING CAST(ENCODE(domain_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_phone_number
		ALTER COLUMN user_id TYPE UUID
		USING CAST(ENCODE(user_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_personal_email
		ALTER COLUMN user_id TYPE UUID
		USING CAST(ENCODE(user_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_personal_email
		ALTER COLUMN domain_id TYPE UUID
		USING CAST(ENCODE(domain_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_unverified_phone_number
		ALTER COLUMN user_id TYPE UUID
		USING CAST(ENCODE(user_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		ALTER COLUMN backup_email_domain_id TYPE UUID
		USING CAST(ENCODE(backup_email_domain_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn alter_workspace_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN super_admin_id TYPE UUID
		USING CAST(ENCODE(super_admin_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	alter_domain_tables(&mut *connection).await?;
	alter_docker_registry_tables(&mut *connection).await?;
	alter_infrastructure_tables(&mut *connection).await?;

	Ok(())
}

async fn alter_domain_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE domain
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE personal_domain
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn alter_docker_registry_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ALTER COLUMN workspace_id TYPE UUID
		USING CAST(ENCODE(workspace_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn alter_infrastructure_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	alter_deployment_tables(&mut *connection).await?;
	alter_managed_database_tables(&mut *connection).await?;
	alter_static_site_tables(&mut *connection).await?;

	Ok(())
}

async fn alter_deployment_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ALTER COLUMN repository_id TYPE UUID
		USING CAST(ENCODE(repository_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		ALTER COLUMN workspace_id TYPE UUID
		USING CAST(ENCODE(workspace_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_environment_variable
		ALTER COLUMN deployment_id TYPE UUID
		USING CAST(ENCODE(deployment_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_request_logs
		ALTER COLUMN deployment_id TYPE UUID
		USING CAST(ENCODE(deployment_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployed_domain
		ALTER COLUMN deployment_id TYPE UUID
		USING CAST(ENCODE(deployment_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployed_domain
		ALTER COLUMN static_site_id TYPE UUID
		USING CAST(ENCODE(static_site_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn alter_managed_database_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE managed_database
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		ALTER COLUMN workspace_id TYPE UUID
		USING CAST(ENCODE(workspace_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn alter_static_site_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ALTER COLUMN id TYPE UUID
		USING CAST(ENCODE(id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_static_sites
		ALTER COLUMN workspace_id TYPE UUID
		USING CAST(ENCODE(workspace_id, 'hex') AS UUID);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn mark_all_non_deferrable_constraints_non_deferrable(
	connection: &mut <Database as sqlx::Database>::Connection,
	constraint_list: &[(String, String)],
) -> Result<(), sqlx::Error> {
	for (table_name, constraint_name) in constraint_list {
		sqlx::query(&format!(
			r#"
			ALTER TABLE {}
			ALTER CONSTRAINT {}
			NOT DEFERRABLE;
			"#,
			table_name, constraint_name,
		))
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
