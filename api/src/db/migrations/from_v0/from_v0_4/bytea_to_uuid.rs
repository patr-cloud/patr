use crate::{migrate_query as query, Database};

/**
 * (
 *  constraint_name,
 *  deferrable,
 *  (
 *  from_table,
 *  from_column,
 *  to_table,
 *  to_column,
 *  )
 * )
 */
const ALL_FOREIGN_KEY_CONSTRAINTS: [(&str, bool, (&str, &str, &str, &str));
	45] = [
	(
		"user_login_fk_user_id",
		false,
		("user_login", "user_id", "\"user\"", "id"),
	),
	(
		"password_reset_request_fk_user_id",
		false,
		("password_reset_request", "user_id", "\"user\"", "id"),
	),
	(
		"workspace_fk_super_admin_id",
		false,
		("workspace", "super_admin_id", "\"user\"", "id"),
	),
	(
		"workspace_domain_fk_id_domain_type",
		false,
		("workspace_domain", "id, domain_type", "domain", "id, type"),
	),
	(
		"personal_domain_fk_id_domain_type",
		false,
		("personal_domain", "id, domain_type", "domain", "id, type"),
	),
	(
		"docker_registry_repository_fk_workspace_id",
		false,
		(
			"docker_registry_repository",
			"workspace_id",
			"workspace",
			"id",
		),
	),
	(
		"deployment_fk_repository_id",
		false,
		(
			"deployment",
			"repository_id",
			"docker_registry_repository",
			"id",
		),
	),
	(
		"deployment_environment_variable_fk_deployment_id",
		false,
		(
			"deployment_environment_variable",
			"deployment_id",
			"deployment",
			"id",
		),
	),
	(
		"deployment_request_logs_fk_deployment_id",
		false,
		(
			"deployment_request_logs",
			"deployment_id",
			"deployment",
			"id",
		),
	),
	(
		"resource_fk_resource_type_id",
		false,
		("resource", "resource_type_id", "resource_type", "id"),
	),
	(
		"resource_fk_owner_id",
		true,
		("resource", "owner_id", "workspace", "id"),
	),
	(
		"role_fk_owner_id",
		false,
		("role", "owner_id", "workspace", "id"),
	),
	(
		"workspace_user_fk_user_id",
		false,
		("workspace_user", "user_id", "\"user\"", "id"),
	),
	(
		"workspace_user_fk_workspace_id",
		false,
		("workspace_user", "workspace_id", "workspace", "id"),
	),
	(
		"workspace_user_fk_role_id",
		false,
		("workspace_user", "role_id", "role", "id"),
	),
	(
		"role_permissions_resource_type_fk_role_id",
		false,
		("role_permissions_resource_type", "role_id", "role", "id"),
	),
	(
		"role_permissions_resource_type_fk_permission_id",
		false,
		(
			"role_permissions_resource_type",
			"permission_id",
			"permission",
			"id",
		),
	),
	(
		"role_permissions_resource_type_fk_resource_type_id",
		false,
		(
			"role_permissions_resource_type",
			"resource_type_id",
			"resource_type",
			"id",
		),
	),
	(
		"role_permissions_resource_fk_role_id",
		false,
		("role_permissions_resource", "role_id", "role", "id"),
	),
	(
		"role_permissions_resource_fk_permission_id",
		false,
		(
			"role_permissions_resource",
			"permission_id",
			"permission",
			"id",
		),
	),
	(
		"role_permissions_resource_fk_resource_id",
		false,
		("role_permissions_resource", "resource_id", "resource", "id"),
	),
	(
		"workspace_fk_id",
		true,
		("workspace", "id", "resource", "id"),
	),
	(
		"workspace_domain_fk_id",
		false,
		("workspace_domain", "id", "domain", "id"),
	),
	(
		"docker_registry_repository_fk_id_workspace_id",
		false,
		(
			"docker_registry_repository",
			"id, workspace_id",
			"resource",
			"id, owner_id",
		),
	),
	(
		"deployment_fk_id_workspace_id",
		false,
		("deployment", "id, workspace_id", "resource", "id, owner_id"),
	),
	(
		"deployment_fk_id_domain_name",
		true,
		(
			"deployment",
			"id, domain_name",
			"deployed_domain",
			"deployment_id, domain_name",
		),
	),
	(
		"deployed_domain_fk_deployment_id_domain_name",
		true,
		(
			"deployed_domain",
			"deployment_id, domain_name",
			"deployment",
			"id, domain_name",
		),
	),
	(
		"deployed_domain_fk_static_site_id_domain_name",
		true,
		(
			"deployed_domain",
			"static_site_id, domain_name",
			"deployment_static_sites",
			"id, domain_name",
		),
	),
	(
		"managed_database_fk_id_workspace_id",
		false,
		(
			"managed_database",
			"id, workspace_id",
			"resource",
			"id, owner_id",
		),
	),
	(
		"deployment_static_sites_fk_id_workspace_id",
		false,
		(
			"deployment_static_sites",
			"id, workspace_id",
			"resource",
			"id, owner_id",
		),
	),
	(
		"deployment_static_sites_fk_id_domain_name",
		true,
		(
			"deployment_static_sites",
			"id, domain_name",
			"deployed_domain",
			"static_site_id, domain_name",
		),
	),
	(
		"personal_email_fk_user_id",
		true,
		("personal_email", "user_id", "\"user\"", "id"),
	),
	(
		"personal_email_fk_domain_id",
		false,
		("personal_email", "domain_id", "personal_domain", "id"),
	),
	(
		"business_email_fk_user_id",
		false,
		("business_email", "user_id", "\"user\"", "id"),
	),
	(
		"business_email_fk_domain_id",
		false,
		("business_email", "domain_id", "workspace_domain", "id"),
	),
	(
		"user_phone_number_fk_user_id",
		true,
		("user_phone_number", "user_id", "\"user\"", "id"),
	),
	(
		"user_phone_number_fk_country_code",
		false,
		(
			"user_phone_number",
			"country_code",
			"phone_number_country_code",
			"country_code",
		),
	),
	(
		"user_unverified_personal_email_fk_domain_id",
		false,
		(
			"user_unverified_personal_email",
			"domain_id",
			"personal_domain",
			"id",
		),
	),
	(
		"user_unverified_personal_email_fk_user_id",
		false,
		(
			"user_unverified_personal_email",
			"user_id",
			"\"user\"",
			"id",
		),
	),
	(
		"user_unverified_phone_number_fk_country_code",
		false,
		(
			"user_unverified_phone_number",
			"country_code",
			"phone_number_country_code",
			"country_code",
		),
	),
	(
		"user_unverified_phone_number_fk_user_id",
		false,
		("user_unverified_phone_number", "user_id", "\"user\"", "id"),
	),
	(
		"user_to_sign_up_fk_backup_email_domain_id",
		false,
		(
			"user_to_sign_up",
			"backup_email_domain_id",
			"personal_domain",
			"id",
		),
	),
	(
		"user_to_sign_up_fk_backup_phone_country_code",
		false,
		(
			"user_to_sign_up",
			"backup_phone_country_code",
			"phone_number_country_code",
			"country_code",
		),
	),
	(
		"user_fk_id_backup_email_local_backup_email_domain_id",
		true,
		(
			"\"user\"",
			"id, backup_email_local, backup_email_domain_id",
			"personal_email",
			"user_id, local, domain_id",
		),
	),
	(
		"user_fk_id_backup_phone_country_code_backup_phone_number",
		true,
		(
			"\"user\"",
			"id, backup_phone_country_code, backup_phone_number",
			"user_phone_number",
			"user_id, country_code, number",
		),
	),
];

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	remove_all_foreign_key_constraints(&mut *connection).await?;
	alter_user_tables(&mut *connection).await?;
	alter_workspace_tables(&mut *connection).await?;
	alter_rbac_tables(&mut *connection).await?;
	add_all_foreign_key_constraints(&mut *connection).await?;

	Ok(())
}

async fn remove_all_foreign_key_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	for (constraint_name, _, (table_name, ..)) in ALL_FOREIGN_KEY_CONSTRAINTS {
		let query = format!(
			"ALTER TABLE {} DROP CONSTRAINT {};",
			table_name, constraint_name
		);
		log::info!(target: "api::queries::migrations", "{}", query);
		sqlx::query(&query).execute(&mut *connection).await?;
	}

	Ok(())
}

async fn alter_rbac_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE resource_type
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

async fn add_all_foreign_key_constraints(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	for (
		constraint_name,
		deferrable,
		(from_table, from_column, to_table, to_column),
	) in ALL_FOREIGN_KEY_CONSTRAINTS
	{
		let query = format!(
			"ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY({}) REFERENCES {}({}){};",
			from_table,
			constraint_name,
			from_column,
			to_table,
			to_column,
			if deferrable {
				" DEFERRABLE INITIALLY IMMEDIATE"
			} else {
				""
			}
		);
		log::info!(target: "api::queries::migrations", "{}", query);
		sqlx::query(&query).execute(&mut *connection).await?;
	}

	Ok(())
}
