use std::collections::{BTreeMap, BTreeSet};

use api_models::{models::workspace::ResourcePermissionType, utils::Uuid};

use super::Role;
use crate::{
	db::{PermissionType, User},
	query,
	query_as,
	Database,
};

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

pub async fn get_permissions_for_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<BTreeMap<Uuid, ResourcePermissionType>, sqlx::Error> {
	let mut permission_set = BTreeMap::new();

	query!(
		r#"
		SELECT
			permission_id as "permission_id: Uuid",
			resource_id as "resource_id: Uuid"
		FROM
			role_resource_permissions_include
		WHERE
			role_id = $1;
		"#,
		role_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.permission_id, row.resource_id))
	.fold(
		BTreeMap::<Uuid, BTreeSet<Uuid>>::new(),
		|mut accu, (permission_id, resource_id)| {
			accu.entry(permission_id).or_default().insert(resource_id);
			accu
		},
	)
	.into_iter()
	.for_each(|(permission_id, resource_ids)| {
		permission_set.insert(
			permission_id,
			ResourcePermissionType::Include(resource_ids),
		);
	});

	query!(
		r#"
		SELECT
			permission_id as "permission_id: Uuid",
			resource_id as "resource_id: Uuid"
		FROM
			role_resource_permissions_exclude
		WHERE
			role_id = $1;
		"#,
		role_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.permission_id, row.resource_id))
	.fold(
		BTreeMap::<Uuid, BTreeSet<Uuid>>::new(),
		|mut accu, (permission_id, resource_id)| {
			accu.entry(permission_id).or_default().insert(resource_id);
			accu
		},
	)
	.into_iter()
	.for_each(|(permission_id, resource_ids)| {
		permission_set.insert(
			permission_id,
			ResourcePermissionType::Exclude(resource_ids),
		);
	});

	query!(
		r#"
		SELECT
			permission_id as "permission_id: Uuid",
			permission_type as "permission_type: PermissionType"
		FROM
			role_resource_permissions_type
		WHERE
			role_id = $1;
		"#,
		role_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.permission_id, row.permission_type))
	.for_each(|(permission_id, permission_type)| {
		permission_set.entry(permission_id).or_insert_with(|| {
			match permission_type {
				PermissionType::Include => {
					ResourcePermissionType::Include(BTreeSet::new())
				}
				PermissionType::Exclude => {
					ResourcePermissionType::Exclude(BTreeSet::new())
				}
			}
		});
	});

	Ok(permission_set)
}

pub async fn remove_all_permissions_for_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			role_resource_permissions_include
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
			role_resource_permissions_exclude
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
			role_resource_permissions_type
		WHERE
			role_id = $1;
		"#,
		role_id as _
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn insert_permissions_for_role(
	connection: &mut <Database as sqlx::Database>::Connection,
	role_id: &Uuid,
	permissions: &BTreeMap<Uuid, ResourcePermissionType>,
) -> Result<(), sqlx::Error> {
	for (permission_id, resource_permission_type) in permissions {
		match resource_permission_type {
			ResourcePermissionType::Include(resource_ids) => {
				query!(
					r#"
					INSERT INTO
						role_resource_permissions_type(
							role_id,
							permission_id,
							permission_type
						)
					VALUES
						($1, $2, $3);
					"#,
					role_id as _,
					permission_id as _,
					PermissionType::Include as _,
				)
				.execute(&mut *connection)
				.await?;

				for resource_id in resource_ids {
					query!(
						r#"
						INSERT INTO
							role_resource_permissions_include(
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
			ResourcePermissionType::Exclude(resource_ids) => {
				query!(
					r#"
					INSERT INTO
						role_resource_permissions_type(
							role_id,
							permission_id,
							permission_type
						)
					VALUES
						($1, $2, $3);
					"#,
					role_id as _,
					permission_id as _,
					PermissionType::Exclude as _,
				)
				.execute(&mut *connection)
				.await?;

				for resource_id in resource_ids {
					query!(
						r#"
						INSERT INTO
							role_resource_permissions_exclude(
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
			"user".sign_up_coupon,
			"user".mfa_secret
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
