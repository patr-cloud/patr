use std::collections::{BTreeMap, HashMap, HashSet};

use api_models::{models::user::WorkspacePermission, utils::Uuid};
use chrono::{DateTime, Utc};

use crate::{db::Workspace, models::rbac, query, query_as, Database};

mod role;
mod user;

pub use self::{role::*, user::*};

pub struct ResourceType {
	pub id: Uuid,
	pub name: String,
	pub description: String,
}

pub struct Resource {
	pub id: Uuid,
	pub resource_type_id: Uuid,
	pub owner_id: Uuid,
	pub created: DateTime<Utc>,
}

pub struct Role {
	pub id: Uuid,
	pub name: String,
	pub description: String,
	pub owner_id: Uuid,
}

pub struct Permission {
	pub id: Uuid,
	pub name: String,
	pub description: String,
}

pub struct WorkspaceUser {
	pub user_id: Uuid,
	pub workspace_id: Uuid,
	pub role_id: Uuid,
}

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
			description VARCHAR(500) NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE resource(
			id UUID CONSTRAINT resource_pk PRIMARY KEY,
			resource_type_id UUID NOT NULL
				CONSTRAINT resource_fk_resource_type_id
					REFERENCES resource_type(id),
			owner_id UUID NOT NULL
				CONSTRAINT resource_fk_owner_id REFERENCES workspace(id)
					DEFERRABLE INITIALLY IMMEDIATE,
			created TIMESTAMPTZ NOT NULL,
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
			description VARCHAR(500) NOT NULL,
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
			name VARCHAR(100) NOT NULL CONSTRAINT permission_uq_name UNIQUE,
			description VARCHAR(500) NOT NULL
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
				permission(
					id,
					name,
					description
				)
			VALUES
				($1, $2, $3);
			"#,
			uuid as _,
			permission,
			"",
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
				resource_type(
					id,
					name,
					description
				)
			VALUES
				($1, $2, $3);
			"#,
			uuid as _,
			resource_type,
			"",
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

pub async fn get_all_workspace_role_permissions_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, sqlx::Error> {
	let mut workspaces: BTreeMap<Uuid, WorkspacePermission> = BTreeMap::new();

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
				if let Some(permissions) = permission
					.resource_permissions
					.get_mut(&resource.resource_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.insert(permission_id);
					}
				} else {
					permission.resource_permissions.insert(
						resource.resource_id,
						HashSet::from([permission_id]),
					);
				}
			}
			for resource_type in resource_types {
				let permission_id = resource_type.permission_id;
				if let Some(permissions) = permission
					.resource_type_permissions
					.get_mut(&resource_type.resource_type_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.insert(permission_id);
					}
				} else {
					permission.resource_type_permissions.insert(
						resource_type.resource_type_id,
						HashSet::from([permission_id]),
					);
				}
			}
		} else {
			let mut permission = WorkspacePermission {
				is_super_admin: false,
				resource_permissions: BTreeMap::new(),
				resource_type_permissions: BTreeMap::new(),
			};
			for resource in resources {
				let permission_id = resource.permission_id;
				if let Some(permissions) = permission
					.resource_permissions
					.get_mut(&resource.resource_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.insert(permission_id);
					}
				} else {
					permission.resource_permissions.insert(
						resource.resource_id,
						HashSet::from([permission_id]),
					);
				}
			}
			for resource_type in resource_types {
				let permission_id = resource_type.permission_id;
				if let Some(permissions) = permission
					.resource_type_permissions
					.get_mut(&resource_type.resource_type_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.insert(permission_id);
					}
				} else {
					permission.resource_type_permissions.insert(
						resource_type.resource_type_id,
						HashSet::from([permission_id]),
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
			name::TEXT as "name!: _",
			super_admin_id as "super_admin_id: _",
			active,
			alert_emails as "alert_emails: _",
			payment_type as "payment_type: _",
			default_payment_method_id as "default_payment_method_id: _",
			deployment_limit,
			static_site_limit,
			database_limit,
			managed_url_limit,
			secret_limit,
			domain_limit,
			docker_repository_storage_limit,
			stripe_customer_id,
			address_id as "address_id: _",
			amount_due
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
				WorkspacePermission {
					is_super_admin: true,
					resource_permissions: BTreeMap::new(),
					resource_type_permissions: BTreeMap::new(),
				},
			);
		}
	}

	Ok(workspaces)
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

pub async fn create_resource(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &Uuid,
	resource_type_id: &Uuid,
	owner_id: &Uuid,
	created: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created
			)
		VALUES
			($1, $2, $3, $4);
		"#,
		resource_id as _,
		resource_type_id as _,
		owner_id as _,
		created
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
	query_as!(
		Resource,
		r#"
		SELECT
			id as "id: _",
			resource_type_id as "resource_type_id: _",
			owner_id as "owner_id: _",
			created 
		FROM
			resource
		WHERE
			id = $1;
		"#,
		resource_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

#[allow(dead_code)]
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
