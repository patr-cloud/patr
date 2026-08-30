//! Normalizes the container-registry schema so a manifest is a generic
//! content-addressed row and the image/index/artifact specifics live in child
//! tables. Before this, `container_registry_manifest` assumed every manifest
//! was a single-platform runnable image (`config_blob_digest`/`platform` were
//! `NOT NULL`), which made multi-arch indexes and OCI artifacts unstorable.
//!
//! What changes:
//! - `blob`/`manifest` digest CHECK relaxed to the general OCI form (sha256, sha512, …); size CHECK
//!   `> 0` → `>= 0` (the OCI empty blob is valid).
//! - `manifest`: `content_type` → `media_type`; add `kind` (image|index|artifact), `artifact_type`,
//!   `subject_digest`; drop `config_blob_digest`/`platform` (moved to `manifest_image`).
//! - new `manifest_image` (config + structured platform) and `manifest_layer` (ordered layers)
//!   replace `manifest_blob`.
//! - `manifest_reference` gains `ordinal` + descriptor/platform columns and is re-keyed on
//!   `(manifest_digest, ordinal)`.
//! - each child table gains a `manifest_kind` column pinned (CHECK) and joined to the parent via a
//!   composite FK on `(digest, kind)`, so a subtype row can only attach to a manifest of the right
//!   kind (image↔manifest_image, …).
//! - `repository_tag` PK reordered to `(repository_id, name)`.
//! - reverse-lookup indices added.
//!
//! Backfill note: because the old schema rejected indexes/artifacts, every
//! existing manifest row is a single image, so `manifest_image` is backfilled
//! from all rows and `manifest_reference` is effectively empty.
//! `manifest_layer` is reconstructed from `manifest_blob` minus each manifest's
//! config blob; layer order and media type aren't recorded in the old table, so
//! we assign a deterministic ordinal and a default gzip layer media type (the
//! manifest bytes in S3 remain the source of truth).

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE container_registry_blob
			DROP CONSTRAINT container_registry_blob_chk_digest,
			ADD CONSTRAINT container_registry_blob_chk_digest
				CHECK(digest ~ '^[a-z0-9]+([+._-][a-z0-9]+)*:[a-f0-9]+$'),
			DROP CONSTRAINT container_registry_blob_chk_size_positive,
			ADD CONSTRAINT container_registry_blob_chk_size_non_negative
				CHECK(size >= 0);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest
			RENAME COLUMN content_type TO media_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TYPE CONTAINER_REGISTRY_MANIFEST_KIND AS ENUM(
			'image', 'index', 'artifact'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest
			ADD COLUMN kind CONTAINER_REGISTRY_MANIFEST_KIND,
			ADD COLUMN artifact_type TEXT,
			ADD COLUMN subject_digest TEXT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE container_registry_manifest
		SET kind = (CASE
			WHEN media_type IN (
				'application/vnd.oci.image.index.v1+json',
				'application/vnd.docker.distribution.manifest.list.v2+json'
			) THEN 'index'
			ELSE 'image'
		END)::CONTAINER_REGISTRY_MANIFEST_KIND;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest
			ALTER COLUMN kind SET NOT NULL,
			ADD CONSTRAINT container_registry_manifest_uq_digest_kind
				UNIQUE(digest, kind),
			DROP CONSTRAINT container_registry_manifest_chk_digest,
			ADD CONSTRAINT container_registry_manifest_chk_digest
				CHECK(digest ~ '^[a-z0-9]+([+._-][a-z0-9]+)*:[a-f0-9]+$'),
			DROP CONSTRAINT container_registry_manifest_chk_size_positive,
			ADD CONSTRAINT container_registry_manifest_chk_size_non_negative
				CHECK(size >= 0);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE container_registry_manifest_image(
			manifest_digest TEXT NOT NULL,
			manifest_kind CONTAINER_REGISTRY_MANIFEST_KIND NOT NULL DEFAULT 'image',
			config_blob_digest TEXT NOT NULL,
			os TEXT NOT NULL,
			architecture TEXT NOT NULL,
			variant TEXT,
			os_version TEXT
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO container_registry_manifest_image(
			manifest_digest, config_blob_digest, os, architecture, variant, os_version
		)
		SELECT
			digest,
			config_blob_digest,
			split_part(platform, '/', 1),
			split_part(platform, '/', 2),
			NULLIF(split_part(platform, '/', 3), ''),
			NULL
		FROM container_registry_manifest
		WHERE kind = 'image' AND config_blob_digest IS NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest_image
			ADD CONSTRAINT container_registry_manifest_image_pk
				PRIMARY KEY(manifest_digest),
			ADD CONSTRAINT container_registry_manifest_image_chk_kind
				CHECK(manifest_kind = 'image'),
			ADD CONSTRAINT container_registry_manifest_image_fk_manifest
				FOREIGN KEY(manifest_digest, manifest_kind)
					REFERENCES container_registry_manifest(digest, kind),
			ADD CONSTRAINT container_registry_manifest_image_fk_config_blob_digest
				FOREIGN KEY(config_blob_digest)
					REFERENCES container_registry_blob(digest);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest
			DROP CONSTRAINT container_registry_manifest_fk_config_blob_digest,
			DROP COLUMN config_blob_digest,
			DROP COLUMN platform;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE container_registry_manifest_layer(
			manifest_digest TEXT NOT NULL,
			manifest_kind CONTAINER_REGISTRY_MANIFEST_KIND NOT NULL,
			ordinal INTEGER NOT NULL,
			blob_digest TEXT NOT NULL,
			media_type TEXT NOT NULL,
			size BIGINT NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO container_registry_manifest_layer(
			manifest_digest, manifest_kind, ordinal, blob_digest, media_type, size
		)
		SELECT
			mb.manifest_digest,
			m.kind,
			(ROW_NUMBER() OVER (
				PARTITION BY mb.manifest_digest ORDER BY mb.blob_digest
			))::INTEGER - 1,
			mb.blob_digest,
			'application/vnd.oci.image.layer.v1.tar+gzip',
			b.size
		FROM container_registry_manifest_blob mb
		JOIN container_registry_blob b ON b.digest = mb.blob_digest
		JOIN container_registry_manifest m ON m.digest = mb.manifest_digest
		LEFT JOIN container_registry_manifest_image mi
			ON mi.manifest_digest = mb.manifest_digest
		WHERE mb.blob_digest IS DISTINCT FROM mi.config_blob_digest;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest_layer
			ADD CONSTRAINT container_registry_manifest_layer_pk
				PRIMARY KEY(manifest_digest, ordinal),
			ADD CONSTRAINT container_registry_manifest_layer_chk_kind
				CHECK(manifest_kind IN ('image', 'artifact')),
			ADD CONSTRAINT container_registry_manifest_layer_fk_manifest
				FOREIGN KEY(manifest_digest, manifest_kind)
					REFERENCES container_registry_manifest(digest, kind),
			ADD CONSTRAINT container_registry_manifest_layer_fk_blob_digest
				FOREIGN KEY(blob_digest)
					REFERENCES container_registry_blob(digest);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(r#"DROP TABLE container_registry_manifest_blob;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest_reference
			RENAME COLUMN digest TO manifest_digest;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest_reference
			ADD COLUMN manifest_kind CONTAINER_REGISTRY_MANIFEST_KIND NOT NULL DEFAULT 'index',
			ADD COLUMN ordinal INTEGER,
			ADD COLUMN media_type TEXT,
			ADD COLUMN size BIGINT,
			ADD COLUMN os TEXT,
			ADD COLUMN architecture TEXT,
			ADD COLUMN variant TEXT,
			ADD COLUMN os_version TEXT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		UPDATE container_registry_manifest_reference
		SET ordinal = sub.rn
		FROM (
			SELECT manifest_digest, referenced_digest,
				(ROW_NUMBER() OVER (
					PARTITION BY manifest_digest ORDER BY referenced_digest
				))::INTEGER - 1 AS rn
			FROM container_registry_manifest_reference
		) sub
		WHERE container_registry_manifest_reference.manifest_digest = sub.manifest_digest
			AND container_registry_manifest_reference.referenced_digest = sub.referenced_digest;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest_reference
			ALTER COLUMN ordinal SET NOT NULL,
			DROP CONSTRAINT container_registry_manifest_reference_pk,
			ADD CONSTRAINT container_registry_manifest_reference_pk
				PRIMARY KEY(manifest_digest, ordinal),
			ADD CONSTRAINT container_registry_manifest_reference_chk_kind
				CHECK(manifest_kind = 'index'),
			DROP CONSTRAINT container_registry_manifest_reference_fk_digest,
			ADD CONSTRAINT container_registry_manifest_reference_fk_manifest
				FOREIGN KEY(manifest_digest, manifest_kind)
					REFERENCES container_registry_manifest(digest, kind);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE container_registry_repository_tag
			DROP CONSTRAINT container_registry_repository_tag_pk,
			ADD CONSTRAINT container_registry_repository_tag_pk
				PRIMARY KEY(repository_id, name);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX container_registry_repository_manifest_idx_manifest_digest
		ON container_registry_repository_manifest(manifest_digest);
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

	sqlx::query(
		r#"
		CREATE INDEX container_registry_manifest_layer_idx_blob_digest
		ON container_registry_manifest_layer(blob_digest);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX container_registry_manifest_image_idx_config_blob_digest
		ON container_registry_manifest_image(config_blob_digest);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
