//! Drops stubbed feature tables (secrets, static sites, managed databases) and
//! the deployment env-var `secret_id` column, plus the `proxy_to_static_site`
//! and `proxy_url` managed-URL variants. These features were never finished or
//! aren't pulling weight and are being removed.
//!
//! Postgres can't drop a value from an enum in place, so MANAGED_URL_TYPE is
//! recreated without the killed variants and the column is repointed.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Drop the deployment env-var FK to secret + the secret_id column itself.
	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			DROP CONSTRAINT IF EXISTS deployment_environment_variable_fk_secret_id,
			DROP CONSTRAINT IF EXISTS deployment_env_var_chk_value_secret_id_either_not_null;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			DROP COLUMN IF EXISTS secret_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE deployment_environment_variable
			ALTER COLUMN value SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Drop the secret table.
	sqlx::query(
		r#"
		DROP TABLE IF EXISTS secret;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Drop the static_site tables.
	sqlx::query(
		r#"
		DROP TABLE IF EXISTS static_site_upload_history CASCADE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE IF EXISTS static_site CASCADE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Drop the managed_database tables and types.
	sqlx::query(
		r#"
		DROP TABLE IF EXISTS managed_database;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE IF EXISTS managed_database_plan;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE IF EXISTS MANAGED_DATABASE_STATUS;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE IF EXISTS LEGACY_MANAGED_DATABASE_PLAN;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE IF EXISTS MANAGED_DATABASE_ENGINE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Strip `proxy_to_static_site` from MANAGED_URL_TYPE. Postgres has no
	// `DROP VALUE`, so recreate the enum.
	sqlx::query(
		r#"
		ALTER TABLE managed_url
			DROP CONSTRAINT IF EXISTS managed_url_chk_values_null_or_not_null,
			DROP CONSTRAINT IF EXISTS managed_url_fk_static_site_id_workspace_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM managed_url
		WHERE url_type IN ('proxy_to_static_site', 'proxy_url');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TYPE MANAGED_URL_TYPE RENAME TO MANAGED_URL_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TYPE MANAGED_URL_TYPE AS ENUM(
			'proxy_to_deployment',
			'redirect'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE managed_url
			ALTER COLUMN url_type TYPE MANAGED_URL_TYPE
			USING url_type::TEXT::MANAGED_URL_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE MANAGED_URL_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE managed_url
			DROP COLUMN IF EXISTS static_site_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE managed_url
			ADD CONSTRAINT managed_url_chk_values_null_or_not_null CHECK(
				(
					url_type = 'proxy_to_deployment' AND
					deployment_id IS NOT NULL AND
					port IS NOT NULL AND
					url IS NULL AND
					permanent_redirect IS NULL AND
					http_only IS NULL
				) OR (
					url_type = 'redirect' AND
					deployment_id IS NULL AND
					port IS NULL AND
					url IS NOT NULL AND
					permanent_redirect IS NOT NULL AND
					http_only IS NOT NULL
				)
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Strip the matching RBAC seed rows so cached perm strings can't reference
	// killed permission categories. Order matters: include/exclude composite
	// FK back to the *_type table, which in turn FK's permission(id).
	for table in [
		"role_resource_permissions_include",
		"role_resource_permissions_exclude",
		"role_resource_permissions_type",
		"user_api_token_resource_permissions_include",
		"user_api_token_resource_permissions_exclude",
		"user_api_token_resource_permissions_type",
	] {
		sqlx::query(&format!(
			r#"
			DELETE FROM {table}
			WHERE permission_id IN (
				SELECT id FROM permission
				WHERE name LIKE 'database::%'
				   OR name LIKE 'staticSite::%'
				   OR name LIKE 'secret::%'
				   OR name LIKE 'dnsRecord::%'
			);
			"#,
		))
		.execute(&mut *connection)
		.await?;
	}

	sqlx::query(
		r#"
		DELETE FROM permission
		WHERE name LIKE 'database::%'
		   OR name LIKE 'staticSite::%'
		   OR name LIKE 'secret::%'
		   OR name LIKE 'dnsRecord::%';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM resource_type
		WHERE name IN ('database', 'staticSite', 'secret', 'dnsRecord');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
