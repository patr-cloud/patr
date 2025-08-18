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
			id UUID,
			workspace_id UUID NOT NULL,
			name TEXT NOT NULL,
			created_at TIMESTAMPTZ NOT NULL,
			updated_at TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_manifest_index(
			digest TEXT,
			annotations JSONB NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_manifest(
			digest TEXT,
			annotations JSONB,
			config JSONB NOT NULL,
			platform JSONB NOT NULL,
			size BIGINT NOT NULL,
			created_at TIMESTAMPTZ NOT NULL,
			index_digest TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_repository_index(
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
		CREATE TABLE container_registry_layer_blob(
			digest TEXT,
			ordinal INT NOT NULL,
			size BIGINT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_layer_manifest(
			manifest_digest TEXT NOT NULL,
			layer_blob_digest TEXT NOT NULL
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
		ALTER TABLE container_registry_manifest_index
		ADD CONSTRAINT container_registry_manifest_index_pk
		PRIMARY KEY(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository_index
		ADD CONSTRAINT container_registry_repository_index_pk
		PRIMARY KEY(repository_id, manifest_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_layer_blob
		ADD CONSTRAINT container_registry_layer_blob_pk
		PRIMARY KEY(digest, ordinal);
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
		ALTER TABLE container_registry_manifest
			ADD CONSTRAINT container_registry_manifest_chk_size_positive 
				CHECK(size > 0),
			ADD CONSTRAINT container_registry_manifest_fk_index_digest
				FOREIGN KEY(index_digest) REFERENCES container_registry_manifest_index(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository_index
			ADD CONSTRAINT container_registry_repository_index_fk_repository_id
				FOREIGN KEY(repository_id)
					REFERENCES container_registry_repository(id),
			ADD CONSTRAINT container_registry_repository_index_fk_manifest_digest
				FOREIGN KEY(manifest_digest)
					REFERENCES container_registry_manifest_index(digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_layer_blob
			ADD CONSTRAINT container_registry_layer_blob_chk_size_positive 
				CHECK(size > 0);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_layer_manifest
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

	Ok(())
}
