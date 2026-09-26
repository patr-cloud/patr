//! Brings a migrated database back in line with a fresh install. Fresh
//! databases never run migration bodies, so these went unnoticed until alpha's
//! schema was diffed against a fresh one:
//!
//! - m002 wrote `managed_url_custom_hostname_chk_sub_domain_valid` with doubled backslashes inside
//!   a raw string, so `\\.` wants a literal backslash between labels and every dotted sub domain
//!   (`api.v2`) is rejected.
//! - NOT NULL constraints keep the name they were created with, so the column renames in m006, m014
//!   and m016 left four of them named after the old columns. A database created after those renames
//!   already has the new names, so each rename only runs if the old name is still there.
//! - m006 added `container_registry_manifest_reference.manifest_kind` with `ADD COLUMN`, which put
//!   it after `referenced_digest` instead of beside `manifest_digest`. Postgres can't reorder
//!   columns, so the table is rebuilt. The rows go through a temp table and the original is dropped
//!   first: constraint names are unique per schema, and the old table's NOT NULLs would otherwise
//!   push the new table's generated names onto a numbered suffix.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE managed_url_custom_hostname
			DROP CONSTRAINT IF EXISTS managed_url_custom_hostname_chk_sub_domain_valid,
			ADD CONSTRAINT managed_url_custom_hostname_chk_sub_domain_valid CHECK(
				sub_domain = '@' OR
				sub_domain ~ '^(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])$'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	for (table, old_name, new_name) in [
		(
			"audit_log",
			"audit_log_login_id_not_null",
			"audit_log_actor_client_id_not_null",
		),
		(
			"container_registry_manifest",
			"container_registry_manifest_content_type_not_null",
			"container_registry_manifest_media_type_not_null",
		),
		(
			"resource",
			"resource_owner_id_not_null",
			"resource_workspace_id_not_null",
		),
		(
			"role",
			"role_owner_id_not_null",
			"role_workspace_id_not_null",
		),
	] {
		let exists = sqlx::query(
			r#"
			SELECT
				1
			FROM
				pg_constraint
			WHERE
				conrelid = $1::REGCLASS AND
				conname = $2;
			"#,
		)
		.bind(table)
		.bind(old_name)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if exists {
			sqlx::query(&format!(
				"ALTER TABLE {table} RENAME CONSTRAINT {old_name} TO {new_name};"
			))
			.execute(&mut *connection)
			.await?;
		}
	}

	sqlx::query(
		r#"
		CREATE TEMPORARY TABLE container_registry_manifest_reference_backup AS
		SELECT
			manifest_digest,
			manifest_kind,
			referenced_digest,
			ordinal,
			media_type,
			size,
			os,
			architecture,
			variant,
			os_version
		FROM
			container_registry_manifest_reference;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE container_registry_manifest_reference;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE container_registry_manifest_reference(
			manifest_digest TEXT NOT NULL,
			manifest_kind CONTAINER_REGISTRY_MANIFEST_KIND NOT NULL DEFAULT 'index',
			referenced_digest TEXT NOT NULL,
			ordinal INTEGER NOT NULL,
			media_type TEXT,
			size BIGINT,
			os TEXT,
			architecture TEXT,
			variant TEXT,
			os_version TEXT
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			container_registry_manifest_reference(
				manifest_digest,
				manifest_kind,
				referenced_digest,
				ordinal,
				media_type,
				size,
				os,
				architecture,
				variant,
				os_version
			)
		SELECT
			manifest_digest,
			manifest_kind,
			referenced_digest,
			ordinal,
			media_type,
			size,
			os,
			architecture,
			variant,
			os_version
		FROM
			container_registry_manifest_reference_backup;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TABLE container_registry_manifest_reference_backup;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest_reference
			ADD CONSTRAINT container_registry_manifest_reference_pk
				PRIMARY KEY(manifest_digest, ordinal),
			ADD CONSTRAINT container_registry_manifest_reference_chk_kind
				CHECK(manifest_kind = 'index'),
			ADD CONSTRAINT container_registry_manifest_reference_fk_manifest
				FOREIGN KEY(manifest_digest, manifest_kind)
					REFERENCES container_registry_manifest(digest, kind),
			ADD CONSTRAINT container_registry_manifest_reference_fk_referenced_digest
				FOREIGN KEY(referenced_digest)
					REFERENCES container_registry_manifest(digest);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX container_registry_manifest_reference_idx_referenced_digest
		ON container_registry_manifest_reference(referenced_digest);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
