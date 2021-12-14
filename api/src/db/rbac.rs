use std::collections::HashMap;

use api_models::utils::Uuid;

use crate::{
	models::{
		db_mapping::{Permission, Resource, ResourceType, Role, Workspace},
		rbac::{self, WorkspacePermissions},
	},
	query,
	query_as,
	Database,
};

pub async fn initialize_rbac_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing rbac tables");

	// Resource types, like application, deployment, VM, etc
	query!(
		r#"
		CREATE TABLE resource_type(
			id UUID CONSTRAINT resource_type_pk PRIMARY KEY,
			name VARCHAR(100) NOT NULL CONSTRAINT resource_type_uq_name UNIQUE,
			description VARCHAR(500)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE resource(
			id UUID CONSTRAINT resource_pk PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			resource_type_id UUID NOT NULL
				CONSTRAINT resource_fk_resource_type_id
					REFERENCES resource_type(id),
			owner_id UUID NOT NULL
				CONSTRAINT resource_fk_owner_id REFERENCES workspace(id)
					DEFERRABLE INITIALLY IMMEDIATE,
			created BIGINT NOT NULL
				CONSTRAINT resource_created_chk_unsigned
						CHECK(created >= 0),
			CONSTRAINT resource_uq_id_owner_id UNIQUE(id, owner_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			resource_idx_owner_id
		ON
			resource
		(owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles belong to an workspace
	query!(
		r#"
		CREATE TABLE role(
			id UUID CONSTRAINT role_pk PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500),
			owner_id UUID NOT NULL
				CONSTRAINT role_fk_owner_id REFERENCES workspace(id),
			CONSTRAINT role_uq_name_owner_id UNIQUE(name, owner_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE permission(
			id UUID CONSTRAINT permission_pk PRIMARY KEY,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Users belong to an workspace through a role
	query!(
		r#"
		CREATE TABLE workspace_user(
			user_id UUID NOT NULL
				CONSTRAINT workspace_user_fk_user_id REFERENCES "user"(id),
			workspace_id UUID NOT NULL
				CONSTRAINT workspace_user_fk_workspace_id
					REFERENCES workspace(id),
			role_id UUID NOT NULL
				CONSTRAINT workspace_user_fk_role_id REFERENCES role(id),
			CONSTRAINT workspace_user_pk
				PRIMARY KEY(user_id, workspace_id, role_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_user_idx_user_id
		ON
			workspace_user
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_user_idx_user_id_workspace_id
		ON
			workspace_user
		(user_id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles that have permissions on a resource type
	query!(
		r#"
		CREATE TABLE role_permissions_resource_type(
			role_id UUID
				CONSTRAINT role_permissions_resource_type_fk_role_id
					REFERENCES role(id),
			permission_id UUID
				CONSTRAINT role_permissions_resource_type_fk_permission_id
					REFERENCES permission(id),
			resource_type_id UUID
				CONSTRAINT role_permissions_resource_type_fk_resource_type_id
					REFERENCES resource_type(id),
			CONSTRAINT role_permissions_resource_type_pk
				PRIMARY KEY(role_id, permission_id, resource_type_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_permissions_resource_type_idx_role_id
		ON
			role_permissions_resource_type
		(role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_permissions_resource_type_idx_role_id_resource_type_id
		ON
			role_permissions_resource_type
		(role_id, resource_type_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles that have permissions on a specific resource
	query!(
		r#"
		CREATE TABLE role_permissions_resource(
			role_id UUID
				CONSTRAINT role_permissions_resource_fk_role_id
					REFERENCES role(id),
			permission_id UUID
				CONSTRAINT role_permissions_resource_fk_permission_id
					REFERENCES permission(id),
			resource_id UUID
				CONSTRAINT role_permissions_resource_fk_resource_id
					REFERENCES resource(id),
			CONSTRAINT role_permissions_resource_pk
				PRIMARY KEY(role_id, permission_id, resource_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_permissions_resource_idx_role_id
		ON
			role_permissions_resource
		(role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_permissions_resource_idx_role_id_resource_id
		ON
			role_permissions_resource
		(role_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_rbac_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up rbac tables initialization");
	for (_, permission) in rbac::permissions::consts_iter().iter() {
		let uuid = generate_new_permission_id(&mut *connection).await?;
		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, NULL);
			"#,
			uuid as _,
			permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	let mut resource_types = HashMap::new();
	for (_, resource_type) in rbac::resource_types::consts_iter().iter() {
		let resource_type = resource_type.to_string();
		let uuid = generate_new_resource_type_id(&mut *connection).await?;
		query!(
			r#"
			INSERT INTO
				resource_type
			VALUES
				($1, $2, NULL);
			"#,
			uuid as _,
			resource_type,
		)
		.execute(&mut *connection)
		.await?;
		resource_types.insert(resource_type, uuid);
	}

	rbac::RESOURCE_TYPES
		.set(resource_types)
		.expect("RESOURCE_TYPES is already set");

	Ok(())
}

pub async fn get_all_workspace_roles_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<HashMap<Uuid, WorkspacePermissions>, sqlx::Error> {
	let mut workspaces: HashMap<Uuid, WorkspacePermissions> = HashMap::new();

	let workspace_roles = query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid",
			role_id as "role_id: Uuid"
		FROM
			workspace_user
		WHERE
			user_id = $1
		ORDER BY
			workspace_id;
		"#,
		user_id as _
	)
	.fetch_all(&mut *connection)
	.await?;

	for workspace_role in workspace_roles {
		let workspace_id = workspace_role.workspace_id;

		let resources = query!(
			r#"
			SELECT
				permission_id as "permission_id: Uuid",
				resource_id as "resource_id: Uuid"
			FROM
				role_permissions_resource
			WHERE
				role_id = $1;
			"#,
			workspace_role.role_id as _
		)
		.fetch_all(&mut *connection)
		.await?;

		let resource_types = query!(
			r#"
			SELECT
				permission_id as "permission_id: Uuid",
				resource_type_id as "resource_type_id: Uuid"
			FROM
				role_permissions_resource_type
			WHERE
				role_id = $1;
			"#,
			workspace_role.role_id as _
		)
		.fetch_all(&mut *connection)
		.await?;

		if let Some(permission) = workspaces.get_mut(&workspace_id) {
			for resource in resources {
				let permission_id = resource.permission_id;
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
				let permission_id = resource_type.permission_id;
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
			let mut permission = WorkspacePermissions {
				is_super_admin: false,
				resources: HashMap::new(),
				resource_types: HashMap::new(),
			};
			for resource in resources {
				let permission_id = resource.permission_id;
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
				let permission_id = resource_type.permission_id;
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
			workspaces.insert(workspace_id, permission);
		}
	}

	// add superadmins to the data-structure too
	let workspaces_details = query_as!(
		Workspace,
		r#"
		SELECT
			id as "id: _",
			name as "name: _",
			super_admin_id as "super_admin_id: _",
			active
		FROM
			workspace
		WHERE
			super_admin_id = $1;
		"#,
		user_id as _
	)
	.fetch_all(&mut *connection)
	.await?;

	for workspace_details in workspaces_details {
		let workspace_id = workspace_details.id;
		if let Some(workspace) = workspaces.get_mut(&workspace_id) {
			workspace.is_super_admin = true;
		} else {
			workspaces.insert(
				workspace_id,
				WorkspacePermissions {
					is_super_admin: true,
					resources: HashMap::new(),
					resource_types: HashMap::new(),
				},
			);
		}
	}

	Ok(workspaces)
}

pub async fn generate_new_role_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				role
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn generate_new_permission_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
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
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn generate_new_resource_type_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
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
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn get_all_resource_types(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<ResourceType>, sqlx::Error> {
	query_as!(
		ResourceType,
		r#"
		SELECT
			id as "id: _",
			name,
			description
		FROM
			resource_type;
		"#
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<Permission>, sqlx::Error> {
	query_as!(
		Permission,
		r#"
		SELECT
			id as "id: _",
			name,
			description
		FROM
			permission;
		"#
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_resource_type_for_resource(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &Uuid,
) -> Result<Option<ResourceType>, sqlx::Error> {
	query_as!(
		ResourceType,
		r#"
		SELECT
			resource_type.id as "id: _",
			resource_type.name,
			resource_type.description
		FROM
			resource_type
		INNER JOIN
			resource
		ON
			resource.resource_type_id = resource_type.id
		WHERE
			resource.id = $1;
		"#,
		resource_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn create_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
	name: &str,
	description: Option<&str>,
	owner_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			role
		VALUES
			($1, $2, $3, $4);
		"#,
		role_id as _,
		name,
		description,
		owner_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_resource(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &Uuid,
	resource_name: &str,
	resource_type_id: &Uuid,
	owner_id: &Uuid,
	created: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			resource
		VALUES
			($1, $2, $3, $4, $5);
		"#,
		resource_id as _,
		resource_name,
		resource_type_id as _,
		owner_id as _,
		created as i64
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn generate_new_resource_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource
			WHERE
				id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn get_resource_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &Uuid,
) -> Result<Option<Resource>, sqlx::Error> {
	let resource = query!(
		r#"
		SELECT
			id as "id: Uuid",
			name,
			resource_type_id as "resource_type_id: Uuid",
			owner_id as "owner_id: Uuid",
			created
		FROM
			resource
		WHERE
			id = $1;
		"#,
		resource_id as _
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| Resource {
		id: row.id,
		name: row.name,
		resource_type_id: row.resource_type_id,
		owner_id: row.owner_id,
		created: row.created as u64,
	});

	Ok(resource)
}

pub async fn delete_resource(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			resource
		WHERE
			id = $1;
		"#,
		resource_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_all_workspace_roles(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Role>, sqlx::Error> {
	query_as!(
		Role,
		r#"
		SELECT
			id as "id: _",
			name,
			description,
			owner_id as "owner_id: _"
		FROM
			role
		WHERE
			owner_id = $1;
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_role_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<Option<Role>, sqlx::Error> {
	query_as!(
		Role,
		r#"
		SELECT
			id as "id: _",
			name,
			description,
			owner_id as "owner_id: _"
		FROM
			role
		WHERE
			id = $1;
		"#,
		role_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

/// For a given role, what permissions does it have and on what resources?
/// Returns a HashMap of Resource -> Permission[]
pub async fn get_permissions_on_resources_for_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<HashMap<Uuid, Vec<Permission>>, sqlx::Error> {
	let mut permissions = HashMap::<Uuid, Vec<Permission>>::new();
	let rows = query!(
		r#"
		SELECT
			permission_id as "permission_id: Uuid",
			resource_id as "resource_id: Uuid",
			name,
			description
		FROM
			role_permissions_resource
		INNER JOIN
			permission
		ON
			role_permissions_resource.permission_id = permission.id
		WHERE
			role_permissions_resource.role_id = $1;
		"#,
		role_id as _
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
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<HashMap<Uuid, Vec<Permission>>, sqlx::Error> {
	let mut permissions = HashMap::<Uuid, Vec<Permission>>::new();
	let rows = query!(
		r#"
		SELECT
			permission_id as "permission_id: Uuid",
			resource_type_id as "resource_type_id: Uuid",
			name,
			description
		FROM
			role_permissions_resource_type
		INNER JOIN
			permission
		ON
			role_permissions_resource_type.permission_id = permission.id
		WHERE
			role_permissions_resource_type.role_id = $1;
		"#,
		role_id as _
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
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			role_permissions_resource
		WHERE
			role_id = $1;
		"#,
		role_id as _
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			role_permissions_resource_type
		WHERE
			role_id = $1;
		"#,
		role_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn insert_resource_permissions_for_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
	resource_permissions: &HashMap<Uuid, Vec<Uuid>>,
) -> Result<(), sqlx::Error> {
	for (resource_id, permissions) in resource_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					role_permissions_resource
				VALUES
					($1, $2, $3);
				"#,
				role_id as _,
				permission_id as _,
				resource_id as _,
			)
			.execute(&mut *connection)
			.await?;
		}
	}
	Ok(())
}

pub async fn insert_resource_type_permissions_for_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
	resource_type_permissions: &HashMap<Uuid, Vec<Uuid>>,
) -> Result<(), sqlx::Error> {
	for (resource_id, permissions) in resource_type_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					role_permissions_resource_type
				VALUES
					($1, $2, $3);
				"#,
				role_id as _,
				permission_id as _,
				resource_id as _,
			)
			.execute(&mut *connection)
			.await?;
		}
	}
	Ok(())
}

pub async fn delete_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<(), sqlx::Error> {
	remove_all_permissions_for_role(&mut *connection, role_id).await?;

	query!(
		r#"
		DELETE FROM
			role
		WHERE
			id = $1;
		"#,
		role_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn remove_all_users_from_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			workspace_user
		WHERE
			role_id = $1;
		"#,
		role_id as _
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}
