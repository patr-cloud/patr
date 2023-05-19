use std::collections::{BTreeMap, BTreeSet, HashMap};

use api_models::{
	models::workspace::{ResourcePermissionType, WorkspacePermission},
	utils::Uuid,
};
use chrono::{DateTime, Utc};

use crate::{models::rbac, query, query_as, Database};

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

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "PERMISSION_TYPE", rename_all = "snake_case")]
pub enum PermissionType {
	Include,
	Exclude,
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

	query!(
		r#"
		CREATE TYPE PERMISSION_TYPE AS ENUM(
			'include',
			'exclude'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_type(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL,

			CONSTRAINT role_resource_permissions_type_pk
				PRIMARY KEY(role_id, permission_id),

			CONSTRAINT role_resource_permissions_type_uq
				UNIQUE (role_id, permission_id, permission_type),

			CONSTRAINT role_resource_permissions_type_fk_role_id
				FOREIGN KEY (role_id) REFERENCES role(id),
			CONSTRAINT role_resource_permissions_type_fk_permission_id
				FOREIGN KEY (permission_id) REFERENCES permission(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_include(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,

			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('include') STORED,

			CONSTRAINT role_resource_permissions_include_pk
				PRIMARY KEY(role_id, permission_id, resource_id),

			CONSTRAINT role_resource_permissions_include_fk_parent
				FOREIGN KEY (role_id, permission_id, permission_type)
					REFERENCES role_resource_permissions_type(role_id, permission_id, permission_type),
			CONSTRAINT role_resource_permissions_include_fk_resource
				FOREIGN KEY (resource_id)
					REFERENCES resource(id)			
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_exclude(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,

			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('exclude') STORED,

			CONSTRAINT role_resource_permissions_exclude_pk
				PRIMARY KEY(role_id, permission_id, resource_id),

			CONSTRAINT role_resource_permissions_exclude_fk_parent
				FOREIGN KEY (role_id, permission_id, permission_type)
					REFERENCES role_resource_permissions_type(role_id, permission_id, permission_type),
			CONSTRAINT role_resource_permissions_exclude_fk_resource
				FOREIGN KEY (resource_id)
					REFERENCES resource(id)	
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO: index for rbac tables

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
	let mut workspace_member_permissions =
		BTreeMap::<Uuid, BTreeMap<Uuid, ResourcePermissionType>>::new();

	query!(
		r#"
		SELECT
			workspace_user.workspace_id as "workspace_id: Uuid",
			role_resource_permissions_include.permission_id as "permission_id: Uuid",
			role_resource_permissions_include.resource_id as "resource_id: Uuid"
		FROM
			role_resource_permissions_include
		JOIN
			workspace_user
			ON workspace_user.role_id = role_resource_permissions_include.role_id
		WHERE
			workspace_user.user_id = $1;
		"#,
		user_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.fold(
		BTreeMap::<(Uuid, Uuid), BTreeSet<Uuid>>::new(),
		|mut accu, (workspace_id, permission_id, resource_id)| {
			accu.entry((workspace_id, permission_id))
				.or_default()
				.insert(resource_id);
			accu
		},
	)
	.into_iter()
	.for_each(|((workspace_id, permission_id), resource_ids)| {
		workspace_member_permissions
			.entry(workspace_id)
			.or_default()
			.insert(
				permission_id,
				ResourcePermissionType::Include(resource_ids),
			);
	});

	query!(
		r#"
		SELECT
			workspace_user.workspace_id as "workspace_id: Uuid",
			role_resource_permissions_exclude.permission_id as "permission_id: Uuid",
			role_resource_permissions_exclude.resource_id as "resource_id: Uuid"
		FROM
			role_resource_permissions_exclude
		JOIN
			workspace_user
			ON workspace_user.role_id = role_resource_permissions_exclude.role_id
		WHERE
			workspace_user.user_id = $1;
		"#,
		user_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.fold(
		BTreeMap::<(Uuid, Uuid), BTreeSet<Uuid>>::new(),
		|mut accu, (workspace_id, permission_id, resource_id)| {
			accu.entry((workspace_id, permission_id))
				.or_default()
				.insert(resource_id);
			accu
		},
	)
	.into_iter()
	.for_each(|((workspace_id, permission_id), resource_ids)| {
		workspace_member_permissions
			.entry(workspace_id)
			.or_default()
			.insert(
				permission_id,
				ResourcePermissionType::Exclude(resource_ids),
			);
	});

	query!(
		r#"
		SELECT
			workspace_user.workspace_id as "workspace_id: Uuid",
			role_resource_permissions_type.permission_id as "permission_id: Uuid",
			role_resource_permissions_type.permission_type as "permission_type: PermissionType"
		FROM
			role_resource_permissions_type
		JOIN
			workspace_user
			ON workspace_user.role_id = role_resource_permissions_type.role_id
		WHERE
			workspace_user.user_id = $1;
		"#,
		user_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.permission_type))
	.for_each(|(workspace_id, permission_id, permission_type)| {
		workspace_member_permissions
			.entry(workspace_id)
			.or_default()
			.entry(permission_id)
			.or_insert_with(|| {
				match permission_type {
					PermissionType::Include => ResourcePermissionType::Include(BTreeSet::new()),
					PermissionType::Exclude => ResourcePermissionType::Exclude(BTreeSet::new())
				}
			});
	});

	let mut workspace_permissions = workspace_member_permissions
		.into_iter()
		.map(|(workspace_id, member_permissions)| {
			(
				workspace_id,
				WorkspacePermission::Member(member_permissions),
			)
		})
		.collect::<BTreeMap<_, _>>();

	query!(
		r#"
		SELECT
			id as "workspace_id: Uuid"
		FROM
			workspace
		WHERE
			super_admin_id = $1;
		"#,
		user_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		// override the member roles for workspace with super admin
		// as superadmin has high privilege than member permissions
		workspace_permissions
			.insert(workspace_id, WorkspacePermission::SuperAdmin);
	});

	Ok(workspace_permissions)
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
