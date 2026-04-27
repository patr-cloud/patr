//! Link every runner to a dedicated service account.
//!
//! Adds `runner.service_account_id` and backfills existing rows. For each
//! pre-existing runner we create a per-runner role with the same permission
//! bundle the new setup flow grants (workspace-wide read on Deployment /
//! Database / StaticSite / Volume / ManagedURL / Domain / Secret /
//! ContainerRegistryRepository View+Pull, and `Runner::Execute` scoped to
//! that single runner via an include list), plus a service account assigned
//! to that role.
//!
//! The orphan SA's `token_hash` is a valid Argon2id hash whose pre-image is
//! never recorded. The runner keeps working off its existing user API token;
//! to start using the SA token an operator must call
//! `regenerateServiceAccountToken` on it.

use argon2::{Algorithm, Argon2, PasswordHasher, Version, password_hash::generate_salt};
use sqlx::{Row, types::Uuid};

use crate::prelude::*;

const WORKSPACE_WIDE_PERMISSIONS: &[&str] = &[
	"deployment::view",
	"database::view",
	"staticSite::view",
	"volume::view",
	"managedURL::view",
	"domain::view",
	"containerRegistryRepository::view",
	"containerRegistryRepository::pull",
	"secret::view",
];

const RUNNER_EXECUTE: &str = "runner::execute";

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

	let runners = sqlx::query(
		r#"
		SELECT
			id, name, workspace_id
		FROM
			runner;
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

		// Per-runner role
		let role_id = sqlx::query(
			r#"
			INSERT INTO
				role(id, name, description, owner_id)
			VALUES
				(
					gen_random_uuid(),
					$1,
					$2,
					$3
				)
			RETURNING id;
			"#,
		)
		.bind(format!("runner-{runner_id}"))
		.bind(format!(
			"Auto-generated role for runner '{runner_name}' service account"
		))
		.bind(workspace_id)
		.fetch_one(&mut *connection)
		.await?
		.try_get::<Uuid, _>("id")?;

		// Workspace-wide grants: empty exclude-list = grant on all resources
		for permission_name in WORKSPACE_WIDE_PERMISSIONS {
			sqlx::query(
				r#"
				INSERT INTO
					role_resource_permissions_type(
						role_id,
						permission_id,
						permission_type
					)
				VALUES
					(
						$1,
						(SELECT id FROM permission WHERE name = $2),
						'exclude'
					);
				"#,
			)
			.bind(role_id)
			.bind(permission_name)
			.execute(&mut *connection)
			.await?;
		}

		// Runner::Execute scoped to just this runner
		sqlx::query(
			r#"
			INSERT INTO
				role_resource_permissions_type(
					role_id,
					permission_id,
					permission_type
				)
			VALUES
				(
					$1,
					(SELECT id FROM permission WHERE name = $2),
					'include'
				);
			"#,
		)
		.bind(role_id)
		.bind(RUNNER_EXECUTE)
		.execute(&mut *connection)
		.await?;

		sqlx::query(
			r#"
			INSERT INTO
				role_resource_permissions_include(
					role_id,
					permission_id,
					resource_id
				)
			VALUES
				(
					$1,
					(SELECT id FROM permission WHERE name = $2),
					$3
				);
			"#,
		)
		.bind(role_id)
		.bind(RUNNER_EXECUTE)
		.bind(runner_id)
		.execute(&mut *connection)
		.await?;

		// Resource row for the SA
		let sa_id = sqlx::query(
			r#"
			INSERT INTO
				resource(
					id,
					resource_type_id,
					owner_id,
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

		// Argon2id hash of a random UUID we immediately throw away. Verification
		// at request time uses the configured pepper; this hash was produced
		// without it, so even if someone guessed the pre-image, verify_password
		// would still return false. Operator must regenerate to get a usable
		// token.
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
					name,
					workspace_id,
					created,
					description,
					token_hash
				)
			VALUES
				(
					$1,
					$2,
					$3,
					NOW(),
					$4,
					$5
				);
			"#,
		)
		.bind(sa_id)
		.bind(format!("runner-{runner_id}"))
		.bind(workspace_id)
		.bind(format!(
			"Auto-generated service account for runner '{runner_name}'. \
			Regenerate the token to start using it."
		))
		.bind(&token_hash)
		.execute(&mut *connection)
		.await?;

		sqlx::query(
			r#"
			INSERT INTO
				service_account_role(
					service_account_id,
					workspace_id,
					role_id
				)
			VALUES
				($1, $2, $3);
			"#,
		)
		.bind(sa_id)
		.bind(workspace_id)
		.bind(role_id)
		.execute(&mut *connection)
		.await?;

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
		.bind(sa_id)
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
