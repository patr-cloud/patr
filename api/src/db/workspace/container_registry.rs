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
			created_at TIMESTAMPTZ NOT NULL,
			updated_at TIMESTAMPTZ NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_manifest(
			digest TEXT NOT NULL,
			size BIGINT NOT NULL,
			created_at TIMESTAMPTZ NOT NULL,
			content_type TEXT NOT NULL
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

	// Blob Digest and Size
	query!(
		r#"
		CREATE TABLE container_registry_layer_blob(
			digest TEXT NOT NULL,
			size BIGINT NOT NULL,
			annotations JSONB
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Create Link table between layer blob and manifest
	query!(
		r#"
		CREATE TABLE container_registry_layer_manifest(
			ordinal INT NOT NULL,
			manifest_digest TEXT NOT NULL,
			layer_blob_digest TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_tag(
			name TEXT NOT NULL,
			repository_id UUID NOT NULL,
			manifest_digest TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE container_registry_session_parts AS (
			part_number	INT,
			etag		TEXT
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_session(
			id UUID NOT NULL,
			user_id UUID NOT NULL,
			aws_session_id TEXT,
			blob_digest TEXT,
			current_part INT,
			last_byte INT,
			parts container_registry_session_parts[],
			updated_at TIMESTAMPTZ NOT NULL
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
		PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_manifest
		ADD CONSTRAINT container_registry_manifest_pk
		PRIMARY KEY(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository_manifest
		ADD CONSTRAINT container_registry_repository_manifest_pk
		PRIMARY KEY(repository_id, manifest_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_layer_blob
		ADD CONSTRAINT container_registry_layer_blob_pk
		PRIMARY KEY(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_layer_manifest
		ADD CONSTRAINT container_registry_layer_manifest_pk
		PRIMARY KEY(manifest_digest, layer_blob_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_tag
		ADD CONSTRAINT container_registry_tag_pk
		PRIMARY KEY(name, repository_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_session
		ADD CONSTRAINT container_registry_session_pk
		PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository
			ADD CONSTRAINT container_registry_repository_chk_name
				CHECK(name ~ '[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*(\/[a-z0-9]+((\.|_|__|-+)[a-z0-9]+)*)*'),
			ADD CONSTRAINT container_registry_repository_uq_id_workspace_id
				UNIQUE(id, workspace_id),
			ADD CONSTRAINT container_registry_repository_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted) REFERENCES resource(id, owner_id, deleted);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_manifest
			ADD CONSTRAINT container_registry_manifest_chk_sha_digest
				CHECK(digest ~ '^sha256:[a-f0-9]{64}$'),
			ADD CONSTRAINT container_registry_manifest_chk_size_positive 
				CHECK(size > 0)
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository_manifest
			ADD CONSTRAINT container_registry_repository_manifest_chk_sha_digest
				CHECK(manifest_digest ~ '^sha256:[a-f0-9]{64}$'),
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
		ALTER TABLE container_registry_layer_blob
			ADD CONSTRAINT container_registry_layer_blob_chk_sha_digest
				CHECK(digest ~ '^sha256:[a-f0-9]{64}$'),
			ADD CONSTRAINT container_registry_layer_blob_chk_size_positive
				CHECK(size > 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_layer_manifest
			ADD CONSTRAINT container_registry_layer_manifest_chk_sha_manifest_digest
				CHECK(manifest_digest ~ '^sha256:[a-f0-9]{64}$'),
			ADD CONSTRAINT container_registry_layer_manifest_chk_sha_layer_blob
				CHECK(layer_blob_digest ~ '^sha256:[a-f0-9]{64}$'),
			ADD CONSTRAINT container_registry_layer_manifest_fk_manifest_digest
				FOREIGN KEY(manifest_digest)
					REFERENCES container_registry_manifest(digest),
			ADD CONSTRAINT container_registry_layer_manifest_fk_layer_blob_digest
				FOREIGN KEY(layer_blob_digest)
					REFERENCES container_registry_layer_blob(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_tag
			ADD CONSTRAINT container_registry_tag_chk_name
				CHECK(name ~ '[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}'),
			ADD CONSTRAINT container_registry_tag_fk_repository_id
				FOREIGN KEY(repository_id)
					REFERENCES container_registry_repository(id),
			ADD CONSTRAINT container_registry_tag_fk_manifest_digest
				FOREIGN KEY(manifest_digest)
					REFERENCES container_registry_manifest(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
