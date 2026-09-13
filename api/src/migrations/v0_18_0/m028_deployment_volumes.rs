//! Replaces workspace-level volumes with per-deployment volumes.
//!
//! A volume was a named, sized, workspace-scoped resource with its own CRUD
//! endpoints and RBAC, attached to a deployment by id through
//! `deployment_volume_mount`. None of it ever reached a runner: the Docker
//! runner discarded `volumes` outright, so no attached volume has ever been
//! mounted. It becomes a plain path on the deployment — `volumes` on the wire
//! is now `{ "/data": {} }` — backed by a two-column `deployment_volume`.
//!
//! **Existing mounts are dropped, not migrated.** They never did anything, so
//! turning them into real mounts now would shadow whatever the image has at
//! that path on the next reconcile and could break a deployment that has been
//! running fine for months. Users re-add the paths they actually want.
//!
//! The RBAC surface goes the way of `m012_remove_static_sites_and_databases`:
//! `volume::*` permissions and the `volume` resource type are deleted along
//! with everything that references them, and any role left holding no
//! permissions — the three seeded `Volume: *` roles, plus any custom role built
//! only from them — is deleted with its bindings and invite grants.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	replace_volume_tables(&mut *connection).await?;
	purge_volume_resources(&mut *connection).await?;
	purge_volume_permissions(&mut *connection).await?;
	delete_empty_roles(&mut *connection).await?;

	Ok(())
}

/// Drops both old tables and creates the new `deployment_volume`. Same name,
/// nothing else in common — the old one was the workspace-level volume
/// registry, the new one is the deployment's list of persisted paths.
async fn replace_volume_tables(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Child first: the mount table FKs onto the volume table.
	sqlx::query(
		r#"
		DROP TABLE deployment_volume_mount;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE deployment_volume;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE deployment_volume(
			deployment_id UUID NOT NULL,
			path TEXT NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_volume
			ADD CONSTRAINT deployment_volume_pk PRIMARY KEY(deployment_id, path),
			ADD CONSTRAINT deployment_volume_fk_deployment_id
				FOREIGN KEY(deployment_id) REFERENCES deployment(id),
			ADD CONSTRAINT deployment_volume_chk_path_valid CHECK(
				path ~ '^(/[^/]+)+$' AND
				path !~ '/\.\.?(/|$)' AND
				LENGTH(path) <= 4096
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Deletes every `resource` row of type `volume`, and first everything that
/// points at one: role bindings and invite grants scoped to it, API token
/// permission bindings scoped to it, and its audit log entries.
async fn purge_volume_resources(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DELETE FROM
			role_binding
		WHERE
			scope_id IN (
				SELECT
					resource.id
				FROM
					resource
				JOIN
					resource_type
				ON
					resource.resource_type_id = resource_type.id
				WHERE
					resource_type.name = 'volume'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			workspace_user_invite_role
		WHERE
			scope_id IN (
				SELECT
					resource.id
				FROM
					resource
				JOIN
					resource_type
				ON
					resource.resource_type_id = resource_type.id
				WHERE
					resource_type.name = 'volume'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			user_api_token_permission_binding
		WHERE
			scope_id IN (
				SELECT
					resource.id
				FROM
					resource
				JOIN
					resource_type
				ON
					resource.resource_type_id = resource_type.id
				WHERE
					resource_type.name = 'volume'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			audit_log
		WHERE
			resource_id IN (
				SELECT
					resource.id
				FROM
					resource
				JOIN
					resource_type
				ON
					resource.resource_type_id = resource_type.id
				WHERE
					resource_type.name = 'volume'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			resource
		WHERE
			resource_type_id IN (
				SELECT id FROM resource_type WHERE name = 'volume'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			resource_type
		WHERE
			name = 'volume';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Deletes the `volume::*` permissions and every grant of them. Permission
/// names are the `Display` form of the old enum, so a prefix match catches all
/// four variants.
async fn purge_volume_permissions(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DELETE FROM
			role_permission
		WHERE
			permission_id IN (
				SELECT id FROM permission WHERE name LIKE 'volume::%'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			user_api_token_permission_binding
		WHERE
			permission_id IN (
				SELECT id FROM permission WHERE name LIKE 'volume::%'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			permission
		WHERE
			name LIKE 'volume::%';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Deletes roles that hold no permissions after the purge, and everything that
/// referenced them: their bindings, and their invite grants — an invite left
/// granting no roles at all is deleted outright, since accepting it would grant
/// nothing.
async fn delete_empty_roles(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		DELETE FROM
			role_binding
		WHERE
			role_id IN (
				SELECT
					role.id
				FROM
					role
				LEFT JOIN
					role_permission
				ON
					role_permission.role_id = role.id
				WHERE
					role_permission.role_id IS NULL
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			workspace_user_invite_role
		WHERE
			role_id IN (
				SELECT
					role.id
				FROM
					role
				LEFT JOIN
					role_permission
				ON
					role_permission.role_id = role.id
				WHERE
					role_permission.role_id IS NULL
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			workspace_user_invite
		WHERE
			NOT EXISTS (
				SELECT
					1
				FROM
					workspace_user_invite_role
				WHERE
					workspace_user_invite_role.invite_id = workspace_user_invite.id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM
			role
		WHERE
			NOT EXISTS (
				SELECT
					1
				FROM
					role_permission
				WHERE
					role_permission.role_id = role.id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
