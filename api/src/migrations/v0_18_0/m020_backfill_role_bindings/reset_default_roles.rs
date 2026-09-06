use sqlx::Row as _;

use super::default_roles::DEFAULT_ROLES;
use crate::prelude::*;

/// Gives every workspace exactly the frozen set of default roles, creating any
/// it is missing and resetting the rest to their seeded permissions.
///
/// Matching is by name. A default-named role that targets specific resources
/// is not a default any more — someone narrowed it deliberately — so it is
/// renamed out of the way and kept as a custom role, and the canonical default
/// is created fresh beside it. Overwriting it instead would widen every
/// holder's access from those resources to the whole workspace.
///
/// A role seeded under older naming (`Deployment Viewer`, before the colon)
/// never matches, and migrates untouched as an ordinary custom role.
pub(super) async fn reset_default_roles(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	let workspaces = sqlx::query("SELECT id FROM workspace;")
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.map(|row| row.try_get::<Uuid, _>("id"))
		.collect::<Result<Vec<_>, _>>()?;

	for workspace_id in workspaces {
		for role in DEFAULT_ROLES {
			let existing = find_role(&mut *connection, &workspace_id, role.name).await?;
			let role_id = match existing {
				Some(id) if targets_resources(&mut *connection, &id).await? => {
					rename_to_custom(&mut *connection, &workspace_id, &id, role.name).await?;
					create_role(&mut *connection, &workspace_id, role).await?
				}
				Some(id) => id,
				None => create_role(&mut *connection, &workspace_id, role).await?,
			};

			clear_permissions(&mut *connection, &role_id).await?;
			grant_permissions(&mut *connection, &role_id, role.permissions).await?;
			mark_immutable(&mut *connection, &role_id).await?;
		}
	}

	Ok(())
}

async fn targets_resources(
	connection: &mut DatabaseConnection,
	role_id: &Uuid,
) -> Result<bool, ErrorType> {
	let targeted = sqlx::query(
		r#"
		SELECT
			1 AS present
		FROM
			role_resource_permissions_include
		WHERE
			role_id = $1
		UNION ALL
		SELECT
			1
		FROM
			role_resource_permissions_exclude
		WHERE
			role_id = $1;
		"#,
	)
	.bind(role_id)
	.fetch_optional(&mut *connection)
	.await?;

	Ok(targeted.is_some())
}

/// Renames a narrowed default out of the default namespace, so the canonical
/// one can take its name. `Deployment: Viewer` becomes `Deployment Viewer -
/// custom`, which is also a name a person could have typed — the colon the
/// seeder uses is not allowed by `RESOURCE_NAME_REGEX`.
async fn rename_to_custom(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	role_id: &Uuid,
	default_name: &str,
) -> Result<(), ErrorType> {
	let base = format!("{} - custom", default_name.replace(": ", " "));

	let mut candidate = base.clone();
	let mut suffix = 2;
	while find_role(&mut *connection, workspace_id, &candidate)
		.await?
		.is_some()
	{
		candidate = format!("{base} {suffix}");
		suffix += 1;
	}

	sqlx::query("UPDATE role SET name = $1 WHERE id = $2;")
		.bind(&candidate)
		.bind(role_id)
		.execute(&mut *connection)
		.await?;

	Ok(())
}

async fn find_role(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	name: &str,
) -> Result<Option<Uuid>, ErrorType> {
	let existing = sqlx::query(
		r#"
		SELECT
			id
		FROM
			role
		WHERE
			workspace_id = $1 AND
			name = $2;
		"#,
	)
	.bind(workspace_id)
	.bind(name)
	.fetch_optional(&mut *connection)
	.await?;

	Ok(existing
		.map(|row| row.try_get::<Uuid, _>("id"))
		.transpose()?)
}

async fn create_role(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	role: &super::default_roles::FrozenRole,
) -> Result<Uuid, ErrorType> {
	let role_id = sqlx::query(
		r#"
		INSERT INTO
			resource(id, resource_type_id, workspace_id, created, deleted)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'role'),
				$1,
				NOW(),
				NULL
			)
		RETURNING id;
		"#,
	)
	.bind(workspace_id)
	.fetch_one(&mut *connection)
	.await?
	.try_get::<Uuid, _>("id")?;

	sqlx::query(
		r#"
		INSERT INTO
			role(id, workspace_id, name, description, is_immutable)
		VALUES
			($1, $2, $3, $4, FALSE);
		"#,
	)
	.bind(role_id)
	.bind(workspace_id)
	.bind(role.name)
	.bind(role.description)
	.execute(&mut *connection)
	.await?;

	Ok(role_id)
}

async fn clear_permissions(
	connection: &mut DatabaseConnection,
	role_id: &Uuid,
) -> Result<(), ErrorType> {
	// Children first: both lists carry an FK to the type row.
	sqlx::query(
		r#"
		DELETE FROM
			role_resource_permissions_include
		WHERE
			role_id = $1;
		"#,
	)
	.bind(role_id)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			role_resource_permissions_exclude
		WHERE
			role_id = $1;
		"#,
	)
	.bind(role_id)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			role_resource_permissions_type
		WHERE
			role_id = $1;
		"#,
	)
	.bind(role_id)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn grant_permissions(
	connection: &mut DatabaseConnection,
	role_id: &Uuid,
	permissions: &[&str],
) -> Result<(), ErrorType> {
	// `exclude` with no resources is how the old model spelled workspace-wide.
	sqlx::query(
		r#"
		INSERT INTO
			role_resource_permissions_type(role_id, permission_id, permission_type)
		SELECT
			$1,
			permission.id,
			'exclude'
		FROM
			permission
		WHERE
			permission.name::TEXT = ANY($2);
		"#,
	)
	.bind(role_id)
	.bind(permissions)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn mark_immutable(
	connection: &mut DatabaseConnection,
	role_id: &Uuid,
) -> Result<(), ErrorType> {
	sqlx::query("UPDATE role SET is_immutable = TRUE WHERE id = $1;")
		.bind(role_id)
		.execute(&mut *connection)
		.await?;

	Ok(())
}
