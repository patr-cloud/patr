//! Makes each default role a strict superset of the tier below it.
//!
//! The seeded ladder had three tiers that granted something the tier above
//! did not need and the tier below should never have had:
//!
//! - `Domain: Viewer` and `Managed URL: Viewer` carried `verify`. Verifying is an edit, not a read,
//!   so it moves up to the Editor of each.
//! - `Container Registry: Viewer` carried `pull`. Reading a repository's metadata and pulling its
//!   images are separate grants, so `pull` moves into a new `Container Registry: Pull` tier between
//!   Viewer and Editor.
//! - `Container Registry: Editor` and `Deployment: Editor` carried `create`. Creating a resource is
//!   an admin act; editing an existing one is not.
//!
//! Only immutable roles are touched — a workspace's own roles keep whatever
//! they were given, even where the name collides.
//!
//! Frozen, like [`super::m020_backfill_role_bindings::default_roles`]: the
//! lists below say what this migration decided, not what the live seed in
//! `create_workspace` says today.

use crate::prelude::*;

/// A default role whose permission set this migration rewrites, and the set it
/// ends up with. Roles absent from this list are left alone.
struct RevisedRole {
	/// Matched against `role.name`, for immutable roles only.
	name: &'static str,
	/// Replaces the role's permissions wholesale.
	permissions: &'static [&'static str],
}

/// Every tier whose membership changes, plus the new registry tier.
const REVISED_ROLES: &[RevisedRole] = &[
	RevisedRole {
		name: "Domain: Viewer",
		permissions: &["domain::view"],
	},
	RevisedRole {
		name: "Domain: Editor",
		permissions: &["domain::add", "domain::verify", "domain::view"],
	},
	RevisedRole {
		name: "Managed URL: Viewer",
		permissions: &["managedURL::view"],
	},
	RevisedRole {
		name: "Managed URL: Editor",
		permissions: &[
			"managedURL::add",
			"managedURL::edit",
			"managedURL::verify",
			"managedURL::view",
		],
	},
	RevisedRole {
		name: "Container Registry: Viewer",
		permissions: &["containerRegistryRepository::view"],
	},
	RevisedRole {
		name: "Container Registry: Pull",
		permissions: &[
			"containerRegistryRepository::pull",
			"containerRegistryRepository::view",
		],
	},
	RevisedRole {
		name: "Container Registry: Editor",
		permissions: &[
			"containerRegistryRepository::edit",
			"containerRegistryRepository::pull",
			"containerRegistryRepository::push",
			"containerRegistryRepository::view",
		],
	},
	RevisedRole {
		name: "Deployment: Editor",
		permissions: &[
			"deployment::edit",
			"deployment::start",
			"deployment::stop",
			"deployment::view",
		],
	},
];

/// The one tier that does not exist yet and has to be minted per workspace.
const NEW_ROLE: (&str, &str) = (
	"Container Registry: Pull",
	"Default role: read and pull container registry repositories.",
);

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	create_missing_role(&mut *connection).await?;

	for role in REVISED_ROLES {
		// Drop whatever it holds that the revised tier does not.
		sqlx::query(
			r#"
			DELETE FROM
				role_permission
			WHERE
				role_id IN (
					SELECT id FROM role WHERE name = $1 AND is_immutable = TRUE
				) AND
				permission_id NOT IN (
					SELECT id FROM permission WHERE name::TEXT = ANY($2)
				);
			"#,
		)
		.bind(role.name)
		.bind(role.permissions)
		.execute(&mut *connection)
		.await?;

		// Add whatever the revised tier gained.
		sqlx::query(
			r#"
			INSERT INTO
				role_permission(role_id, permission_id)
			SELECT
				role.id,
				permission.id
			FROM
				role
			CROSS JOIN
				permission
			WHERE
				role.name = $1 AND
				role.is_immutable = TRUE AND
				permission.name::TEXT = ANY($2)
			ON CONFLICT
				(role_id, permission_id)
			DO NOTHING;
			"#,
		)
		.bind(role.name)
		.bind(role.permissions)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

/// Mints `Container Registry: Pull` in every workspace that lacks it. A role is
/// a resource, so it needs a `resource` row before the `role` row can point at
/// it.
async fn create_missing_role(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	let workspaces = sqlx::query_as::<_, (Uuid,)>(
		r#"
		SELECT
			workspace.id
		FROM
			workspace
		WHERE
			NOT EXISTS (
				SELECT 1 FROM role WHERE role.workspace_id = workspace.id AND role.name = $1
			);
		"#,
	)
	.bind(NEW_ROLE.0)
	.fetch_all(&mut *connection)
	.await?;

	for (workspace_id,) in workspaces {
		let (role_id,) = sqlx::query_as::<_, (Uuid,)>(
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
		.await?;

		sqlx::query(
			r#"
			INSERT INTO
				role(id, workspace_id, name, description, is_immutable)
			VALUES
				($1, $2, $3, $4, TRUE);
			"#,
		)
		.bind(role_id)
		.bind(workspace_id)
		.bind(NEW_ROLE.0)
		.bind(NEW_ROLE.1)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
