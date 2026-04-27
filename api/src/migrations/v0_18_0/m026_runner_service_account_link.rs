//! Links every runner to a dedicated service account.
//!
//! Adds `runner.service_account_id` and backfills existing rows. Each runner
//! gets a service account that is its own actor and its own client, plus the
//! two grants the new setup flow issues: `Runner: All Resource Reader` across
//! the workspace, and `Runner: Execute` scoped to that one runner. Two
//! bindings
//! rather than one because a binding carries a single scope — binding the
//! execute role workspace-wide would let every runner act on every other.
//!
//! Both roles are seeded here for workspaces that predate them, marked
//! immutable like the rest of the default catalogue. The permission names are
//! frozen copies: a migration must keep deciding what it decided the day it
//! ran, so renaming a permission later must not change what this granted.
//!
//! The orphan account's `token_hash` is a valid Argon2id hash whose pre-image
//! is never recorded, and it was produced without the configured pepper, so
//! verification would fail even if the pre-image were guessed. The runner
//! keeps working off its existing user API token; to move onto the service
//! account an operator must call `regenerateServiceAccountToken`.

use argon2::{Algorithm, Argon2, PasswordHasher, Version, password_hash::generate_salt};
use sqlx::{Row, types::Uuid};

use crate::prelude::*;

/// Read access to every resource a runner needs to run a deployment. No
/// billing, roles, service accounts or other runners: a runner is
/// infrastructure, not an administrator.
const ALL_RESOURCE_READ_PERMISSIONS: &[&str] = &[
	"containerRegistryRepository::pull",
	"containerRegistryRepository::view",
	"deployment::view",
	"domain::view",
	"managedURL::view",
	"secret::view",
	"volume::view",
];

/// The one permission a runner holds on itself.
const EXECUTE_PERMISSIONS: &[&str] = &["runner::execute"];

/// The roles seeded here, as they were defined when this migration was
/// written.
const RUNNER_ROLES: &[(&str, &str, &[&str])] = &[
	(
		"Runner: All Resource Reader",
		"Default role: read-only access to every resource a runner needs to run a deployment. Granted to a runner's service account across the workspace. Deliberately excludes billing, roles, service accounts and other runners.",
		ALL_RESOURCE_READ_PERMISSIONS,
	),
	(
		"Runner: Execute",
		"Default role: lets a runner act on deployments assigned to it. Granted to a runner's service account, scoped to that one runner.",
		EXECUTE_PERMISSIONS,
	),
];

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE runner
		ADD COLUMN service_account_id UUID;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Seed the two runner roles into every workspace that doesn't have them.
	// A role is itself a resource, so the resource row comes first.
	let workspaces = sqlx::query(
		r#"
		SELECT
			id
		FROM
			workspace;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?;

	for workspace in workspaces {
		let workspace_id: Uuid = workspace.try_get("id")?;

		for (name, description, permissions) in RUNNER_ROLES {
			let role_id = sqlx::query(
				r#"
				INSERT INTO
					resource(
						id,
						resource_type_id,
						workspace_id,
						created
					)
				VALUES
					(
						GENERATE_RESOURCE_ID(),
						(SELECT id FROM resource_type WHERE name = 'role'),
						$1,
						NOW()
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
					role(
						id,
						name,
						description,
						workspace_id,
						is_immutable
					)
				VALUES
					($1, $2, $3, $4, TRUE);
				"#,
			)
			.bind(role_id)
			.bind(name)
			.bind(description)
			.bind(workspace_id)
			.execute(&mut *connection)
			.await?;

			for permission_name in *permissions {
				sqlx::query(
					r#"
					INSERT INTO
						role_permission(
							role_id,
							permission_id
						)
					VALUES
						($1, (SELECT id FROM permission WHERE name = $2));
					"#,
				)
				.bind(role_id)
				.bind(permission_name)
				.execute(&mut *connection)
				.await?;
			}
		}
	}

	let runners = sqlx::query(
		r#"
		SELECT
			runner.id,
			runner.name,
			runner.workspace_id,
			workspace.super_admin_id
		FROM
			runner
		INNER JOIN
			workspace
		ON
			workspace.id = runner.workspace_id;
		"#,
	)
	.fetch_all(&mut *connection)
	.await?;

	let argon2 = Argon2::new(
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	);

	for row in runners {
		let runner_id: Uuid = row.try_get("id")?;
		let runner_name: String = row.try_get("name")?;
		let workspace_id: Uuid = row.try_get("workspace_id")?;
		let super_admin_id: Uuid = row.try_get("super_admin_id")?;

		let service_account_id = sqlx::query(
			r#"
			INSERT INTO
				resource(
					id,
					resource_type_id,
					workspace_id,
					created
				)
			VALUES
				(
					GENERATE_RESOURCE_ID(),
					(SELECT id FROM resource_type WHERE name = 'serviceAccount'),
					$1,
					NOW()
				)
			RETURNING id;
			"#,
		)
		.bind(workspace_id)
		.fetch_one(&mut *connection)
		.await?
		.try_get::<Uuid, _>("id")?;

		// The same id registers the account as a client and as an actor.
		sqlx::query(
			r#"
			INSERT INTO
				actor_client(id, actor_client_type)
			VALUES
				($1, 'service_account');
			"#,
		)
		.bind(service_account_id)
		.execute(&mut *connection)
		.await?;

		sqlx::query(
			r#"
			INSERT INTO
				workspace_actor(id, workspace_id, actor_type)
			VALUES
				($1, $2, 'service_account');
			"#,
		)
		.bind(service_account_id)
		.bind(workspace_id)
		.execute(&mut *connection)
		.await?;

		let unrecoverable_secret = Uuid::new_v4();
		let token_hash = argon2
			.hash_password_with_salt(unrecoverable_secret.as_bytes(), &generate_salt())
			.map_err(ErrorType::server_error)?
			.to_string();

		sqlx::query(
			r#"
			INSERT INTO
				service_account(
					id,
					workspace_id,
					name,
					description,
					token_hash,
					created
				)
			VALUES
				($1, $2, $3, $4, $5, NOW());
			"#,
		)
		.bind(service_account_id)
		.bind(workspace_id)
		.bind(format!("runner-{runner_id}"))
		.bind(format!(
			"Auto-generated service account for runner '{runner_name}'. \
			Regenerate the token to start using it."
		))
		.bind(&token_hash)
		.execute(&mut *connection)
		.await?;

		// Read across the workspace; execute only on this runner.
		for (role_name, scope_id) in [
			("Runner: All Resource Reader", workspace_id),
			("Runner: Execute", runner_id),
		] {
			sqlx::query(
				r#"
				INSERT INTO
					role_binding(
						id,
						workspace_id,
						actor_id,
						role_id,
						scope_id,
						created,
						created_by
					)
				VALUES
					(
						GEN_RANDOM_UUID(),
						$1,
						$2,
						(
							SELECT
								id
							FROM
								role
							WHERE
								workspace_id = $1 AND
								name = $3
						),
						$4,
						NOW(),
						$5
					);
				"#,
			)
			.bind(workspace_id)
			.bind(service_account_id)
			.bind(role_name)
			.bind(scope_id)
			.bind(super_admin_id)
			.execute(&mut *connection)
			.await?;
		}

		sqlx::query(
			r#"
			UPDATE
				runner
			SET
				service_account_id = $1
			WHERE
				id = $2;
			"#,
		)
		.bind(service_account_id)
		.bind(runner_id)
		.execute(&mut *connection)
		.await?;
	}

	sqlx::query(
		r#"
		ALTER TABLE runner
		ALTER COLUMN service_account_id SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE runner
		ADD CONSTRAINT runner_fk_service_account_id
			FOREIGN KEY (service_account_id) REFERENCES service_account(id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
