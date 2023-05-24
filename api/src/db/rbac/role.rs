use std::collections::{BTreeMap, HashMap};

use api_models::utils::Uuid;

use super::{Permission, Role};
use crate::{db::User, query, query_as, Database};

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

pub async fn create_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
	name: &str,
	description: &str,
	owner_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			role(
				id,
				name,
				description,
				owner_id
			)
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

pub async fn get_all_roles_in_workspace(
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
			role_allow_permissions_resource
		INNER JOIN
			permission
		ON
			role_allow_permissions_resource.permission_id = permission.id
		WHERE
			role_allow_permissions_resource.role_id = $1;
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
			role_allow_permissions_resource_type
		INNER JOIN
			permission
		ON
			role_allow_permissions_resource_type.permission_id = permission.id
		WHERE
			role_allow_permissions_resource_type.role_id = $1;
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
			role_allow_permissions_resource
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
			role_allow_permissions_resource_type
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
	resource_permissions: &BTreeMap<Uuid, Vec<Uuid>>,
) -> Result<(), sqlx::Error> {
	for (resource_id, permissions) in resource_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					role_allow_permissions_resource(
						role_id,
						permission_id,
						resource_id
					)
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
	resource_type_permissions: &BTreeMap<Uuid, Vec<Uuid>>,
) -> Result<(), sqlx::Error> {
	for (resource_type_id, permissions) in resource_type_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					role_allow_permissions_resource_type(
						role_id,
						permission_id,
						resource_type_id
					)
				VALUES
					($1, $2, $3);
				"#,
				role_id as _,
				permission_id as _,
				resource_type_id as _,
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

pub async fn get_all_users_with_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<Vec<User>, sqlx::Error> {
	query_as!(
		User,
		r#"
		SELECT
			"user".id as "id: _",
			"user".username,
			"user".password,
			"user".first_name,
			"user".last_name,
			"user".dob,
			"user".bio,
			"user".location,
			"user".created,
			"user".recovery_email_local,
			"user".recovery_email_domain_id as "recovery_email_domain_id: _",
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".workspace_limit,
			"user".sign_up_coupon
		FROM
			"user"
		INNER JOIN
			workspace_user
		ON
			"user".id = workspace_user.user_id
		WHERE
			workspace_user.role_id = $1;
		"#,
		role_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn update_role_name_and_description(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
	name: Option<&str>,
	description: Option<&str>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			role
		SET
			name = COALESCE($1, name),
			description = COALESCE($2, description)
		WHERE
			id = $3;
		"#,
		name,
		description,
		role_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
