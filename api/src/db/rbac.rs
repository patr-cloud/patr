use std::{collections::HashMap, fmt::Write};

use crate::{models::rbac::OrgPermissions, query};

use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_rbac_pre(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing rbac tables");

	// Resource types, like application, deployment, VM, etc
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS resource_type (
			id VARCHAR(100) PRIMARY KEY,
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
			resource_type_id VARCHAR(100) NOT NULL,
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
			id VARCHAR(100) PRIMARY KEY,
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
			permission_id VARCHAR(100),
			resource_type_id VARCHAR(100),
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
			permission_id VARCHAR(100),
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

pub async fn initialize_rbac_post(
	_transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	Ok(())
}

pub async fn get_all_organisation_roles_for_user(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
	user_id: &[u8],
) -> Result<HashMap<String, OrgPermissions>, sqlx::Error> {
	let mut orgs: HashMap<String, OrgPermissions> = HashMap::new();

	let org_roles = query!(
		r#"
		SELECT
			*
		FROM
			organisation_user
		WHERE
			user_id = ?
		ORDER BY
			organisation_id;
		"#,
		user_id
	)
	.fetch_all(&mut *transaction)
	.await?;

	for org_role in org_roles {
		let mut org_id = String::with_capacity(32);
		for byte in org_role.organisation_id {
			write!(org_id, "{:02x}", byte)
				.expect("unable to write byte array to string");
		}

		let resources = query!(
			r#"
			SELECT
				*
			FROM
				role_permissions_resource
			WHERE
				role_id = ?
			"#,
			org_role.role_id
		)
		.fetch_all(&mut *transaction)
		.await?;

		let resource_types = query!(
			r#"
			SELECT
				*
			FROM
				role_permissions_resource_type
			WHERE
				role_id = ?;
			"#,
			org_role.role_id
		)
		.fetch_all(&mut *transaction)
		.await?;

		if let Some(permission) = orgs.get_mut(&org_id) {
			for resource in resources {
				if let Some(permissions) =
					permission.resources.get_mut(&resource.resource_id)
				{
					if !permissions.contains(&resource.permission_id) {
						permissions.push(resource.permission_id);
					}
				} else {
					permission.resources.insert(
						resource.resource_id,
						vec![resource.permission_id],
					);
				}
			}
			for resource_type in resource_types {
				if let Some(permissions) = permission
					.resource_types
					.get_mut(&resource_type.resource_type_id)
				{
					if !permissions.contains(&resource_type.permission_id) {
						permissions.push(resource_type.permission_id);
					}
				} else {
					permission.resource_types.insert(
						resource_type.resource_type_id,
						vec![resource_type.permission_id],
					);
				}
			}
		} else {
			let mut permission = OrgPermissions {
				is_super_admin: false,
				resources: HashMap::new(),
				resource_types: HashMap::new(),
			};
			for resource in resources {
				if let Some(permissions) =
					permission.resources.get_mut(&resource.resource_id)
				{
					if !permissions.contains(&resource.permission_id) {
						permissions.push(resource.permission_id);
					}
				} else {
					permission.resources.insert(
						resource.resource_id,
						vec![resource.permission_id],
					);
				}
			}
			for resource_type in resource_types {
				if let Some(permissions) = permission
					.resource_types
					.get_mut(&resource_type.resource_type_id)
				{
					if !permissions.contains(&resource_type.permission_id) {
						permissions.push(resource_type.permission_id);
					}
				} else {
					permission.resource_types.insert(
						resource_type.resource_type_id,
						vec![resource_type.permission_id],
					);
				}
			}
			orgs.insert(org_id, permission);
		}
	}
	// add superadmins to the data-structure too

	let orgs_details = query!(
		r#"
		SELECT
			*
		FROM
			organisation
		WHERE
			super_admin_id = ?;
		"#,
		user_id
	)
	.fetch_all(&mut *transaction)
	.await?;

	for org_details in orgs_details {
		let mut org_id = String::with_capacity(32);
		for byte in org_details.id {
			write!(org_id, "{:02x}", byte)
				.expect("unable to write byte array to string");
		}
		if let Some(org) = orgs.get_mut(&org_id) {
			org.is_super_admin = true;
		} else {
			orgs.insert(
				org_id,
				OrgPermissions {
					is_super_admin: true,
					resources: HashMap::new(),
					resource_types: HashMap::new(),
				},
			);
		}
	}

	Ok(orgs)
}
