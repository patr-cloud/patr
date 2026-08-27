//! Removes `proxy_to_static_site` from the `MANAGED_URL_TYPE` enum.
//!
//! `m012_remove_static_sites_and_databases` deleted every managed URL of that
//! type and rewrote the CHECK constraint to reject it, but left the value in
//! the enum — Postgres has no `ALTER TYPE ... DROP VALUE`. That left migrated
//! databases carrying a value that a freshly initialised one does not, since
//! `initialize_managed_url_tables` already creates the type without it.
//!
//! That divergence is worth closing on its own: the offline sqlx cache records
//! type metadata, so a fresh database and a migrated one describing the same
//! column differently is exactly the kind of mismatch that only shows up at
//! runtime, on whichever of the two nobody tested against.
//!
//! The type is swapped rather than edited: rename the old one aside, create the
//! replacement, repoint the column with a text round-trip, then drop the
//! original. The CHECK has to come off first — it depends on the column — and
//! goes back on unchanged afterwards.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Belt and braces: m012 already deleted these, but the cast below would
	// fail on any row that somehow still held the value, and failing here with
	// a clear cause beats failing inside ALTER TABLE.
	sqlx::query(
		r#"
		DELETE FROM
			managed_url
		WHERE
			url_type = 'proxy_to_static_site';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE managed_url
			DROP CONSTRAINT managed_url_chk_values_null_or_not_null;
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
			'proxy_url',
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
			ADD CONSTRAINT managed_url_chk_values_null_or_not_null CHECK(
				(
					url_type = 'proxy_to_deployment' AND
					deployment_id IS NOT NULL AND
					port IS NOT NULL AND
					url IS NULL AND
					permanent_redirect IS NULL AND
					http_only IS NULL
				) OR (
					url_type = 'proxy_url' AND
					deployment_id IS NULL AND
					port IS NULL AND
					url IS NOT NULL AND
					permanent_redirect IS NULL AND
					http_only IS NOT NULL
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

	Ok(())
}
