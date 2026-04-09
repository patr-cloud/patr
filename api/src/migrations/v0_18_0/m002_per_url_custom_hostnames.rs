//! Migrate from per-domain custom hostnames to per-URL custom hostnames.
//!
//! - Creates the `managed_url_custom_hostname` table
//! - Moves `cloudflare_custom_hostname_id` from `workspace_domain` into new
//!   per-URL entries (one per distinct sub_domain + domain_id in managed_url)
//! - Drops the old column from `workspace_domain`
//! - Drops `is_active` from `managed_url`
//! - Adds FK from `managed_url` to `managed_url_custom_hostname`

use crate::{prelude::*, utils::config::AppConfig};

#[macros::migration]
async fn migrate(
	connection: &mut DatabaseConnection,
	_config: &AppConfig,
) -> Result<(), ErrorType> {
	// 1. Create the new table
	sqlx::query(
		r#"
		CREATE TABLE managed_url_custom_hostname(
			sub_domain TEXT NOT NULL,
			domain_id UUID NOT NULL,
			cloudflare_custom_hostname_id TEXT NOT NULL,
			is_active BOOLEAN NOT NULL DEFAULT FALSE,
			last_verified TIMESTAMPTZ
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE managed_url_custom_hostname
			ADD CONSTRAINT managed_url_custom_hostname_pk
				PRIMARY KEY(sub_domain, domain_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE managed_url_custom_hostname
			ADD CONSTRAINT managed_url_custom_hostname_fk_domain_id
				FOREIGN KEY(domain_id)
					REFERENCES workspace_domain(id),
			ADD CONSTRAINT managed_url_custom_hostname_chk_sub_domain_valid CHECK(
				sub_domain = '@' OR
				sub_domain ~ '^(([a-z0-9_]|[a-z0-9_][a-z0-9_\\-]*[a-z0-9_])\\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\\-]*[a-z0-9_])$'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// 2. Migrate existing custom hostnames: for each distinct (sub_domain,
	//    domain_id) in managed_url that has a non-deleted domain with a
	//    cloudflare_custom_hostname_id, insert into the new table.
	sqlx::query(
		"INSERT INTO managed_url_custom_hostname(
			sub_domain, domain_id, cloudflare_custom_hostname_id, is_active
		)
		SELECT DISTINCT
			mu.sub_domain,
			mu.domain_id,
			wd.cloudflare_custom_hostname_id,
			TRUE
		FROM managed_url mu
		JOIN workspace_domain wd ON wd.id = mu.domain_id
		WHERE mu.deleted IS NULL
			AND wd.deleted IS NULL
			AND wd.cloudflare_custom_hostname_id IS NOT NULL
		ON CONFLICT DO NOTHING;",
	)
	.execute(&mut *connection)
	.await?;

	// 3. Drop the old column from workspace_domain
	sqlx::query(
		"ALTER TABLE workspace_domain
			DROP COLUMN IF EXISTS cloudflare_custom_hostname_id;",
	)
	.execute(&mut *connection)
	.await?;

	// 4. Drop is_active from managed_url
	sqlx::query(
		"ALTER TABLE managed_url
			DROP COLUMN IF EXISTS is_active;",
	)
	.execute(&mut *connection)
	.await?;

	// 5. Add FK from managed_url to managed_url_custom_hostname
	sqlx::query(
		"ALTER TABLE managed_url
			ADD CONSTRAINT managed_url_fk_custom_hostname
				FOREIGN KEY(sub_domain, domain_id)
					REFERENCES managed_url_custom_hostname(sub_domain, domain_id);",
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
