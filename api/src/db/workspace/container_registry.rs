use crate::prelude::*;

/// Initializes all container registry related tables
#[instrument(skip(connection))]
pub async fn initialize_container_registry_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up container registry tables");
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

	// Blob Digest and Size
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

	query!(
		r#"
		CREATE TABLE container_registry_manifest(
			digest TEXT NOT NULL,
			content_type TEXT NOT NULL,
			size BIGINT NOT NULL,
			config_blob_digest TEXT NOT NULL,
			platform TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// If a manifest references another manifest, this table will store that
	// reference. This is needed to handle the case where a manifest references
	// another manifest (eg: Index manifests)
	query!(
		r#"
		CREATE TABLE container_registry_manifest_reference(
			digest TEXT NOT NULL,
			referenced_digest TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Create Link table between blob and manifest
	query!(
		r#"
		CREATE TABLE container_registry_manifest_blob(
			manifest_digest TEXT NOT NULL,
			blob_digest TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Link table between repository and manifest
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

	query!(
		r#"
		ALTER TABLE container_registry_blob
			ADD CONSTRAINT container_registry_blob_pk
				PRIMARY KEY(digest),
			ADD CONSTRAINT container_registry_blob_chk_digest
				CHECK(digest ~ '^sha256:[a-f0-9]{64}$'),
			ADD CONSTRAINT container_registry_blob_chk_size_positive
				CHECK(size > 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_manifest
			ADD CONSTRAINT container_registry_manifest_pk
				PRIMARY KEY(digest),
			ADD CONSTRAINT container_registry_manifest_chk_digest
				CHECK(digest ~ '^sha256:[a-f0-9]{64}$'),
			ADD CONSTRAINT container_registry_manifest_chk_size_positive
				CHECK(size > 0),
			ADD CONSTRAINT container_registry_manifest_fk_config_blob_digest
				FOREIGN KEY(config_blob_digest)
					REFERENCES container_registry_blob(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_manifest_reference
			ADD CONSTRAINT container_registry_manifest_reference_pk
				PRIMARY KEY(digest, referenced_digest),
			ADD CONSTRAINT container_registry_manifest_reference_fk_digest
				FOREIGN KEY(digest)
					REFERENCES container_registry_manifest(digest),
			ADD CONSTRAINT container_registry_manifest_reference_fk_referenced_digest
				FOREIGN KEY(referenced_digest)
					REFERENCES container_registry_manifest(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_manifest_blob
			ADD CONSTRAINT container_registry_manifest_blob_pk
				PRIMARY KEY(manifest_digest, blob_digest),
			ADD CONSTRAINT container_registry_manifest_blob_fk_manifest_digest
				FOREIGN KEY(manifest_digest)
					REFERENCES container_registry_manifest(digest),
			ADD CONSTRAINT container_registry_manifest_blob_fk_blob_digest
				FOREIGN KEY(blob_digest)
					REFERENCES container_registry_blob(digest);
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

	query!(
		r#"
		ALTER TABLE container_registry_repository_tag
			ADD CONSTRAINT container_registry_repository_tag_pk
				PRIMARY KEY(name, repository_id),
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
#[instrument(skip(_connection))]
pub async fn initialize_container_registry_indices(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up container registry indices");

	Ok(())
}
