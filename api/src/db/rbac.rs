use std::collections::HashMap;

use sqlx::{MySql, Transaction};
use uuid::Uuid;

use crate::{
	models::{
		db_mapping::{Permission, Resource, ResourceType, Role},
		rbac::{self, OrgPermissions},
	},
	query,
	query_as,
};

pub async fn initialize_rbac_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing rbac tables");

	// Resource types, like application, deployment, VM, etc
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
			name VARCHAR(100) NOT NULL,
			resource_type_id BINARY(16) NOT NULL,
			owner_id BINARY(16),
			FOREIGN KEY(owner_id) REFERENCES organisation(id),
			FOREIGN KEY(resource_type_id) REFERENCES resource_type(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	// Roles belong to an organisation
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS role (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500),
			owner_id BINARY(16) NOT NULL,
			UNIQUE(name, owner_id),
			FOREIGN KEY(owner_id) REFERENCES organisation(id)
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

pub async fn initialize_rbac_post(
	connection: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	for (_, permission) in rbac::permissions::consts_iter().iter() {
		let uuid = generate_new_resource_id(&mut *connection).await?;
		let uuid = uuid.as_bytes().as_ref();
		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				(?, ?, NULL)
			"#,
			uuid,
			permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	let resource_types = rbac::resource_types::consts_iter()
		.iter()
		.map(|(_, resource_type)| {
			(
				resource_type.to_string(),
				Uuid::new_v4().as_bytes().to_vec(),
			)
		})
		.collect::<HashMap<_, _>>();

	for (resource_type, uuid) in &resource_types {
		query!(
			r#"
			INSERT INTO
				resource_type
			VALUES
				(?, ?, NULL);
			"#,
			uuid,
			resource_type,
		)
		.execute(&mut *connection)
		.await?;
	}

	rbac::RESOURCE_TYPES
		.set(resource_types)
		.expect("RESOURCE_TYPES is already set");

	Ok(())
}

pub async fn get_all_organisation_roles_for_user(
	transaction: &mut Transaction<'_, MySql>,
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
		let org_id = hex::encode(org_role.organisation_id);

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
				let permission_id = hex::encode(&resource.permission_id);
				if let Some(permissions) =
					permission.resources.get_mut(&resource.resource_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					permission
						.resources
						.insert(resource.resource_id, vec![permission_id]);
				}
			}
			for resource_type in resource_types {
				let permission_id = hex::encode(&resource_type.permission_id);
				if let Some(permissions) = permission
					.resource_types
					.get_mut(&resource_type.resource_type_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					permission.resource_types.insert(
						resource_type.resource_type_id,
						vec![permission_id],
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
				let permission_id = hex::encode(&resource.permission_id);
				if let Some(permissions) =
					permission.resources.get_mut(&resource.resource_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					permission
						.resources
						.insert(resource.resource_id, vec![permission_id]);
				}
			}
			for resource_type in resource_types {
				let permission_id = hex::encode(&resource_type.permission_id);
				if let Some(permissions) = permission
					.resource_types
					.get_mut(&resource_type.resource_type_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					permission.resource_types.insert(
						resource_type.resource_type_id,
						vec![permission_id],
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
		let org_id = hex::encode(org_details.id);
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

pub async fn generate_new_role_id(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut rows = query_as!(
		Role,
		r#"
		SELECT
			*
		FROM
			role
		WHERE
			id = ?;
		"#,
		uuid.as_bytes().as_ref()
	)
	.fetch_all(&mut *connection)
	.await?;

	while !rows.is_empty() {
		uuid = Uuid::new_v4();
		rows = query_as!(
			Role,
			r#"
			SELECT
				*
			FROM
				role
			WHERE
				id = ?;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?;
	}

	Ok(uuid)
}

pub async fn get_all_resource_types(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Vec<ResourceType>, sqlx::Error> {
	query_as!(
		ResourceType,
		r#"
		SELECT
			*
		FROM
			resource_type;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn get_all_permissions(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Vec<Permission>, sqlx::Error> {
	query_as!(
		Permission,
		r#"
		SELECT
			*
		FROM
			permission;
		"#
	)
	.fetch_all(connection)
	.await
}

pub async fn get_resource_type_for_resource(
	connection: &mut Transaction<'_, MySql>,
	resource_id: &[u8],
) -> Result<Option<ResourceType>, sqlx::Error> {
	let rows = query_as!(
		ResourceType,
		r#"
		SELECT
			resource_type.*
		FROM
			resource_type
		INNER JOIN
			resource
		ON
			resource.resource_type_id = resource_type.id
		WHERE
			resource.id = ?;
		"#,
		resource_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn create_role(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
	name: &str,
	description: Option<&str>,
	owner_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			role
		VALUES
			(?, ?, ?, ?);
		"#,
		role_id,
		name,
		description,
		owner_id
	)
	.fetch_all(connection)
	.await?;
	Ok(())
}

pub async fn create_orphaned_resource(
	connection: &mut Transaction<'_, MySql>,
	resource_id: &[u8],
	resource_name: &str,
	resource_type_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			resource
		VALUES
			(?, ?, ?, NULL);
		"#,
		resource_id,
		resource_name,
		resource_type_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn set_resource_owner_id(
	connection: &mut Transaction<'_, MySql>,
	resource_id: &[u8],
	owner_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			resource
		SET
			owner_id = ?
		WHERE
			id = ?;
		"#,
		owner_id,
		resource_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn create_resource(
	connection: &mut Transaction<'_, MySql>,
	resource_id: &[u8],
	resource_name: &str,
	resource_type_id: &[u8],
	owner_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			resource
		VALUES
			(?, ?, ?, ?);
		"#,
		resource_id,
		resource_name,
		resource_type_id,
		owner_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn generate_new_resource_id(
	connection: &mut Transaction<'_, MySql>,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut rows = query_as!(
		Resource,
		r#"
		SELECT
			id,
			name,
			resource_type_id,
			owner_id as `owner_id!`
		FROM
			resource
		WHERE
			id = ?;
		"#,
		uuid.as_bytes().as_ref()
	)
	.fetch_all(&mut *connection)
	.await?;

	while !rows.is_empty() {
		uuid = Uuid::new_v4();
		rows = query_as!(
			Resource,
			r#"
			SELECT
				id,
				name,
				resource_type_id,
				owner_id as `owner_id!`
			FROM
				resource
			WHERE
				id = ?;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?;
	}

	Ok(uuid)
}

pub async fn get_resource_by_id(
	connection: &mut Transaction<'_, MySql>,
	resource_id: &[u8],
) -> Result<Option<Resource>, sqlx::Error> {
	let rows = query_as!(
		Resource,
		r#"
		SELECT
			id,
			name,
			resource_type_id,
			owner_id as `owner_id!`
		FROM
			resource
		WHERE
			id = ?;
		"#,
		resource_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows.into_iter().next())
}

pub async fn delete_resource(
	connection: &mut Transaction<'_, MySql>,
	resource_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			resource
		WHERE
			id = ?;
		"#,
		resource_id
	)
	.execute(connection)
	.await?;

	Ok(())
}

pub async fn get_all_organisation_roles(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<Role>, sqlx::Error> {
	query_as!(
		Role,
		r#"
		SELECT
			*
		FROM
			role
		WHERE
			owner_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}

pub async fn get_role_by_id(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
) -> Result<Option<Role>, sqlx::Error> {
	let rows = query_as!(
		Role,
		r#"
		SELECT
			*
		FROM
			role
		WHERE
			id = ?;
		"#,
		role_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

/// For a given role, what permissions does it have and on what resources?
/// Returns a HashMap of Resource -> Permission[]
pub async fn get_permissions_on_resources_for_role(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
) -> Result<HashMap<Vec<u8>, Vec<Permission>>, sqlx::Error> {
	let mut permissions = HashMap::<Vec<u8>, Vec<Permission>>::new();
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			role_permissions_resource
		INNER JOIN
			permission
		ON
			role_permissions_resource.permission_id = permission.id
		WHERE
			role_permissions_resource.role_id = ?;
		"#,
		role_id
	)
	.fetch_all(&mut *connection)
	.await?;

	for row in rows {
		let permission = Permission {
			id: row.permission_id,
			name: row.name,
			description: row.description,
		};
		permissions
			.entry(row.resource_id)
			.or_insert_with(Vec::new)
			.push(permission);
	}

	Ok(permissions)
}

/// For a given role, what permissions does it have and on what resources types?
/// Returns a HashMap of ResourceType -> Permission[]
pub async fn get_permissions_on_resource_types_for_role(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
) -> Result<HashMap<Vec<u8>, Vec<Permission>>, sqlx::Error> {
	let mut permissions = HashMap::<Vec<u8>, Vec<Permission>>::new();
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			role_permissions_resource_type
		INNER JOIN
			permission
		ON
			role_permissions_resource_type.permission_id = permission.id
		WHERE
			role_permissions_resource_type.role_id = ?;
		"#,
		role_id
	)
	.fetch_all(&mut *connection)
	.await?;

	for row in rows {
		let permission = Permission {
			id: row.permission_id,
			name: row.name,
			description: row.description,
		};
		permissions
			.entry(row.resource_type_id)
			.or_insert_with(Vec::new)
			.push(permission);
	}

	Ok(permissions)
}

pub async fn remove_all_permissions_for_role(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			role_permissions_resource
		WHERE
			role_id = ?;
		"#,
		role_id
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			role_permissions_resource_type
		WHERE
			role_id = ?;
		"#,
		role_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn insert_resource_permissions_for_role(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
	resource_permissions: &HashMap<Vec<u8>, Vec<Vec<u8>>>,
) -> Result<(), sqlx::Error> {
	for (resource_id, permissions) in resource_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					role_permissions_resource
				VALUES
					(?, ?, ?);
				"#,
				role_id,
				permission_id,
				resource_id,
			)
			.execute(&mut *connection)
			.await?;
		}
	}
	Ok(())
}

pub async fn insert_resource_type_permissions_for_role(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
	resource_type_permissions: &HashMap<Vec<u8>, Vec<Vec<u8>>>,
) -> Result<(), sqlx::Error> {
	for (resource_id, permissions) in resource_type_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					role_permissions_resource_type
				VALUES
					(?, ?, ?);
				"#,
				role_id,
				permission_id,
				resource_id,
			)
			.execute(&mut *connection)
			.await?;
		}
	}
	Ok(())
}

pub async fn delete_role(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
) -> Result<(), sqlx::Error> {
	remove_all_permissions_for_role(&mut *connection, role_id).await?;

	query!(
		r#"
		DELETE FROM
			role
		WHERE
			id = ?;
		"#,
		role_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn remove_all_users_from_role(
	connection: &mut Transaction<'_, MySql>,
	role_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			organisation_user
		WHERE
			role_id = ?;
		"#,
		role_id
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
