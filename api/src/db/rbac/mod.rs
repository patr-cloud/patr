use std::collections::HashMap;

use api_models::utils::Uuid;

use crate::{
	db::Workspace,
	models::rbac::{self, WorkspacePermissions},
	query,
	query_as,
	Database,
};

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
	pub name: String,
	pub resource_type_id: Uuid,
	pub owner_id: Uuid,
	pub created: u64,
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
		CREATE TABLE role_allow_permissions_resource_type(
			role_id UUID
				CONSTRAINT role_allow_permissions_resource_type_fk_role_id
					REFERENCES role(id),
			permission_id UUID
				CONSTRAINT role_allow_permissions_resource_type_fk_permission_id
					REFERENCES permission(id),
			resource_type_id UUID
				CONSTRAINT role_allow_permissions_resource_type_fk_resource_type_id
					REFERENCES resource_type(id),
			CONSTRAINT role_allow_permissions_resource_type_pk
				PRIMARY KEY(role_id, permission_id, resource_type_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_allow_permissions_resource_type_idx_role_id
		ON
			role_allow_permissions_resource_type
		(role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_allow_permissions_resource_type_idx_roleid_resourcetypeid
		ON
			role_allow_permissions_resource_type
		(role_id, resource_type_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles that have permissions on a specific resource
	query!(
		r#"
		CREATE TABLE role_allow_permissions_resource(
			role_id UUID
				CONSTRAINT role_allow_permissions_resource_fk_role_id
					REFERENCES role(id),
			permission_id UUID
				CONSTRAINT role_allow_permissions_resource_fk_permission_id
					REFERENCES permission(id),
			resource_id UUID
				CONSTRAINT role_allow_permissions_resource_fk_resource_id
					REFERENCES resource(id),
			CONSTRAINT role_allow_permissions_resource_pk
				PRIMARY KEY(role_id, permission_id, resource_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_allow_permissions_resource_idx_role_id
		ON
			role_allow_permissions_resource
		(role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_allow_permissions_resource_idx_role_id_resource_id
		ON
			role_allow_permissions_resource
		(role_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

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

	add_validation_for_permissions_on_resource(&mut *connection).await?;

	Ok(())
}

async fn add_validation_for_permissions_on_resource(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
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
							'workspace::project::info'
							'workspace::project::delete'
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

		let blocked_resources = query!(
			r#"
			SELECT
				permission_id as "permission_id: Uuid",
				resource_id as "resource_id: Uuid"
			FROM
				role_block_permissions_resource
			WHERE
				role_id = $1;
			"#,
			workspace_role.role_id as _
		)
		.fetch_all(&mut *connection)
		.await?;

		let resources = query!(
			r#"
			SELECT
				permission_id as "permission_id: Uuid",
				resource_id as "resource_id: Uuid"
			FROM
				role_allow_permissions_resource
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
				role_allow_permissions_resource_type
			WHERE
				role_id = $1;
			"#,
			workspace_role.role_id as _
		)
		.fetch_all(&mut *connection)
		.await?;

		if let Some(workspace_permissions) = workspaces.get_mut(&workspace_id) {
			for blocked_resource in blocked_resources {
				let permission_id = blocked_resource.permission_id;
				if let Some(permissions) = workspace_permissions
					.blocked_resources
					.get_mut(&blocked_resource.resource_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					workspace_permissions.blocked_resources.insert(
						blocked_resource.resource_id,
						vec![permission_id],
					);
				}
			}
			for resource in resources {
				let permission_id = resource.permission_id;
				if let Some(permissions) = workspace_permissions
					.allowed_resources
					.get_mut(&resource.resource_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					workspace_permissions
						.allowed_resources
						.insert(resource.resource_id, vec![permission_id]);
				}
			}
			for resource_type in resource_types {
				let permission_id = resource_type.permission_id;
				if let Some(permissions) = workspace_permissions
					.allowed_resource_types
					.get_mut(&resource_type.resource_type_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					workspace_permissions.allowed_resource_types.insert(
						resource_type.resource_type_id,
						vec![permission_id],
					);
				}
			}
		} else {
			let mut permission = WorkspacePermissions {
				is_super_admin: false,
				blocked_resources: HashMap::new(),
				allowed_resources: HashMap::new(),
				allowed_resource_types: HashMap::new(),
			};
			for blocked_resource in blocked_resources {
				let permission_id = blocked_resource.permission_id;
				if let Some(permissions) = permission
					.blocked_resources
					.get_mut(&blocked_resource.resource_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					permission.blocked_resources.insert(
						blocked_resource.resource_id,
						vec![permission_id],
					);
				}
			}
			for resource in resources {
				let permission_id = resource.permission_id;
				if let Some(permissions) =
					permission.allowed_resources.get_mut(&resource.resource_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					permission
						.allowed_resources
						.insert(resource.resource_id, vec![permission_id]);
				}
			}
			for resource_type in resource_types {
				let permission_id = resource_type.permission_id;
				if let Some(permissions) = permission
					.allowed_resource_types
					.get_mut(&resource_type.resource_type_id)
				{
					if !permissions.contains(&permission_id) {
						permissions.push(permission_id);
					}
				} else {
					permission.allowed_resource_types.insert(
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
			name::TEXT as "name!: _",
			super_admin_id as "super_admin_id: _",
			active,
			alert_emails
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
					blocked_resources: HashMap::new(),
					allowed_resources: HashMap::new(),
					allowed_resource_types: HashMap::new(),
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
	resource_name: &str,
	resource_type_id: &Uuid,
	owner_id: &Uuid,
	created: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			resource(
				id,
				name,
				resource_type_id,
				owner_id,
				created
			)
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

pub async fn update_resource_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &Uuid,
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			resource
		SET
			name = $2
		WHERE
			id = $1;
		"#,
		resource_id as _,
		name
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
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
