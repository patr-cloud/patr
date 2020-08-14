use crate::query;

use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_rbac(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	// Resource types, like Application, deployment, VM, etc
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS resource_type (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) UNIQUE NOT NULL,
			description VARCHAR(500)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS resource (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100),
			resource_type_id BINARY(16) NOT NULL,
			owner_id BINARY(16) NOT NULL,
			FOREIGN KEY(owner_id) REFERENCES organisation(id),
			FOREIGN KEY(resource_type_id) REFERENCES resource_type(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS role (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS permission (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// Users belong to an organisation through a role
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS organisation_user (
			user_id BINARY(16) NOT NULL,
			organisation_id BINARY(16) NOT NULL,
			role_id BINARY(16) NOT NULL,
			PRIMARY KEY(user_id, organisation_id, role_id),
			FOREIGN KEY(user_id) REFERENCES user(id),
			FOREIGN KEY(organisation_id) REFERENCES organisation(id),
			FOREIGN KEY(role_id) REFERENCES role(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// Roles that have permissions on a resource type
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS role_permissions_resource_type (
			role_id BINARY(16),
			permission_id BINARY(16),
			resource_type_id BINARY(16),
			PRIMARY KEY(role_id, permission_id, resource_type_id),
			FOREIGN KEY(role_id) REFERENCES role(id),
			FOREIGN KEY(permission_id) REFERENCES permission(id),
			FOREIGN KEY(resource_type_id) REFERENCES resource_type(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// Roles that have permissions on a specific resource
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS role_permissions_resource (
			role_id BINARY(16),
			permission_id BINARY(16),
			resource_id BINARY(16),
			PRIMARY KEY(role_id, permission_id, resource_id),
			FOREIGN KEY(role_id) REFERENCES role(id),
			FOREIGN KEY(permission_id) REFERENCES permission(id),
			FOREIGN KEY(resource_id) REFERENCES resource(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}
