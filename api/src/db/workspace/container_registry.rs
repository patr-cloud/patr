use crate::prelude::*;

/// Initializes all container registry related tables
#[instrument(skip(connection))]
pub async fn initialize_container_registry_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up container registry tables");

	// A repository is a namespace that holds container images, e.g. `myapp` in
	// `registry.patr.cloud/<workspace>/myapp`. Owned by a workspace.
	query!(
		r#"
		CREATE TABLE container_registry_repository(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			name TEXT NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// A blob - globally addressable by its content digest - is a chunk of data
	// stored in the registry. `size` is the byte length.
	query!(
		r#"
		CREATE TABLE container_registry_blob(
			digest TEXT NOT NULL,
			size BIGINT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Which of the three shapes a manifest takes. Set by the application at push
	// time (the image-vs-artifact call is a heuristic, so it is not tied to the
	// child tables by a DB constraint).
	query!(
		r#"
		CREATE TYPE CONTAINER_REGISTRY_MANIFEST_KIND AS ENUM(
			'image', /* A runnable single-platform image */
			'index', /* A multi-arch index / manifest list referencing child images */
			'artifact' /* An OCI artifact (SBOM, signature, …); also the catch-all */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// A manifest is the JSON document a client pushes/pulls. It describes ONE of:
	// a runnable image, a multi-arch index (a list of image manifests), or an
	// artifact (e.g. an SBOM or signature). Keyed by its content digest and
	// global (deduplicated registry-wide). `kind` says which of the three it is;
	// `artifact_type`/`subject_digest` are OCI 1.1 fields (null for plain images).
	query!(
		r#"
		CREATE TABLE container_registry_manifest(
			digest TEXT NOT NULL,
			media_type TEXT NOT NULL,
			size BIGINT NOT NULL,
			kind CONTAINER_REGISTRY_MANIFEST_KIND NOT NULL,
			artifact_type TEXT,
			subject_digest TEXT
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Image-only metadata for a runnable image manifest: a pointer to its config
	// blob, and the platform (os/architecture) it runs on. Exactly one row per
	// image manifest; absent for indexes and artifacts. `manifest_kind` is pinned
	// to 'image' (CHECK + composite FK) so this row can only attach to an
	// image-kind manifest.
	query!(
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
		"#
	)
	.execute(&mut *connection)
	.await?;

	// The ordered filesystem layers of an image or artifact manifest, each
	// pointing at a stored blob. `ordinal` preserves layer order (0, 1, 2, …).
	// `media_type` records the layer type. Foreign/non-distributable layers are
	// rejected at push, so every layer's blob is guaranteed present (hence the
	// hard FK). `manifest_kind` is pinned to 'image' or 'artifact' (CHECK +
	// composite FK), so layers can never attach to an index-kind manifest.
	query!(
		r#"
		CREATE TABLE container_registry_manifest_layer(
			manifest_digest TEXT NOT NULL,
			manifest_kind CONTAINER_REGISTRY_MANIFEST_KIND NOT NULL,
			ordinal INTEGER NOT NULL,
			blob_digest TEXT NOT NULL,
			media_type TEXT NOT NULL,
			size BIGINT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// For a multi-arch index (or a nested index): the child manifests it bundles,
	// with each child's platform copied from the index descriptor. Lets us list
	// an image's architectures directly, and enforces that children exist.
	// `manifest_kind` is pinned to 'index' (CHECK + composite FK), so only an
	// index-kind manifest can have children; `referenced_digest` (the child) is a
	// plain FK — a child may be any kind (image, nested index, or artifact).
	query!(
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
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Which repositories contain which manifests. Manifests are global/deduped,
	// so this link table is what scopes a manifest to a repository (and carries
	// the push time within that repo).
	query!(
		r#"
		CREATE TABLE container_registry_repository_manifest(
			repository_id UUID NOT NULL,
			manifest_digest TEXT NOT NULL,
			created_at TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Human-readable tags (`latest`, `v1.2`) pointing at a manifest digest within
	// a repository. Tags move between digests over time; the digest itself never
	// changes.
	query!(
		r#"
		CREATE TABLE container_registry_repository_tag(
			name TEXT NOT NULL,
			repository_id UUID NOT NULL,
			manifest_digest TEXT NOT NULL,
			last_updated TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes all container registry related constraints
#[instrument(skip(connection))]
pub async fn initialize_container_registry_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up container registry constraints");

	query!(
		r#"
		ALTER TABLE container_registry_repository
			ADD CONSTRAINT container_registry_repository_pk
				PRIMARY KEY(id),
			ADD CONSTRAINT container_registry_repository_uq_id_workspace_id
				UNIQUE(id, workspace_id),
			ADD CONSTRAINT container_registry_repository_uq_name_workspace_id
				UNIQUE(name, workspace_id),
			ADD CONSTRAINT container_registry_repository_fk_workspace_id
				FOREIGN KEY(workspace_id)
					REFERENCES workspace(id),
			ADD CONSTRAINT container_registry_repository_chk_name CHECK(
				name ~ '^[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*(\/[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*)*$'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Digest CHECK accepts the general OCI digest form (sha256, sha512, …) rather
	// than sha256-only. Size CHECK allows 0 so the OCI empty blob is storable.
	query!(
		r#"
		ALTER TABLE container_registry_blob
			ADD CONSTRAINT container_registry_blob_pk
				PRIMARY KEY(digest),
			ADD CONSTRAINT container_registry_blob_chk_digest
				CHECK(digest ~ '^[a-z0-9]+([+._-][a-z0-9]+)*:[a-f0-9]+$'),
			ADD CONSTRAINT container_registry_blob_chk_size_non_negative
				CHECK(size >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// `kind` is a native enum (CONTAINER_REGISTRY_MANIFEST_KIND) so the value
	// domain is enforced by the type. The UNIQUE(digest, kind) is the FK target
	// the child tables reference to pin each subtype (an image only gets a
	// manifest_image row, only an index gets references, …) — see their composite
	// FKs below. `subject_digest` is a soft ref (no FK): the referrers spec
	// permits a dangling subject.
	query!(
		r#"
		ALTER TABLE container_registry_manifest
			ADD CONSTRAINT container_registry_manifest_pk
				PRIMARY KEY(digest),
			ADD CONSTRAINT container_registry_manifest_uq_digest_kind
				UNIQUE(digest, kind),
			ADD CONSTRAINT container_registry_manifest_chk_digest
				CHECK(digest ~ '^[a-z0-9]+([+._-][a-z0-9]+)*:[a-f0-9]+$'),
			ADD CONSTRAINT container_registry_manifest_chk_size_non_negative
				CHECK(size >= 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
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
		"#
	)
	.execute(&mut *connection)
	.await?;

	// `blob_digest` is a hard FK: foreign/non-distributable layers are rejected at
	// push, so every layer blob is present in the registry.
	query!(
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
		"#
	)
	.execute(&mut *connection)
	.await?;

	// `referenced_digest` FK (default NO ACTION) enforces that an index's children
	// exist before it's pushed, and blocks deleting a child while an index still
	// references it.
	query!(
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
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository_manifest
			ADD CONSTRAINT container_registry_repository_manifest_pk
				PRIMARY KEY(repository_id, manifest_digest),
			ADD CONSTRAINT container_registry_repository_manifest_fk_repository_id
				FOREIGN KEY(repository_id)
					REFERENCES container_registry_repository(id),
			ADD CONSTRAINT container_registry_repository_manifest_fk_manifest_digest
				FOREIGN KEY(manifest_digest)
					REFERENCES container_registry_manifest(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// PK is (repository_id, name) — repository-first — so per-repo lexical tag
	// listing (OCI `GET /tags/list` with `?n=`/`?last=`) is indexed for free.
	query!(
		r#"
		ALTER TABLE container_registry_repository_tag
			ADD CONSTRAINT container_registry_repository_tag_pk
				PRIMARY KEY(repository_id, name),
			ADD CONSTRAINT container_registry_repository_tag_fk_repository_id
				FOREIGN KEY(repository_id)
					REFERENCES container_registry_repository(id),
			ADD CONSTRAINT container_registry_repository_tag_fk_manifest_digest
				FOREIGN KEY(repository_id, manifest_digest)
					REFERENCES container_registry_repository_manifest(
						repository_id,
						manifest_digest
					);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes all container registry related indices
#[instrument(skip(connection))]
pub async fn initialize_container_registry_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up container registry indices");

	// Reverse-lookup indices: "which repos hold manifest X", "what references
	// child Y", "what layer/config uses blob Z". None of these are covered by the
	// primary keys above.
	query!(
		r#"
		CREATE INDEX container_registry_repository_manifest_idx_manifest_digest
		ON container_registry_repository_manifest(manifest_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX container_registry_manifest_reference_idx_referenced_digest
		ON container_registry_manifest_reference(referenced_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX container_registry_manifest_layer_idx_blob_digest
		ON container_registry_manifest_layer(blob_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX container_registry_manifest_image_idx_config_blob_digest
		ON container_registry_manifest_image(config_blob_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
