use crate::prelude::*;

/// Initializes all container registry related tables
#[instrument(skip(connection))]
pub async fn initialize_container_registry_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up container registry tables");
	query!(
		r#"
			CREATE TABLE container_registry_repository (
				id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
				workspace_id UUID NOT NULL,
				name TEXT NOT NULL,
				created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
				updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
			CREATE TABLE container_registry_index (
				digest TEXT PRIMARY KEY,
				annotations JSONB DEFAULT NULL
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
			CREATE TABLE container_registry_manifest (
				digest TEXT PRIMARY KEY,
				annotations JSONB DEFAULT NULL,
				config JSONB NOT NULL,
				platform JSONB NOT NULL,
				size BIGINT NOT NULL,
				created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
				index_digest TEXT NOT NULL,

				FOREIGN KEY (index_digest) REFERENCES container_registry_index(digest)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
			CREATE TABLE container_registry_repository_index (
				repository_id UUID NOT NULL,
				manifest_digest TEXT NOT NULL,
				created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

				PRIMARY KEY (repository_id, manifest_digest),

				FOREIGN KEY (repository_id) REFERENCES container_registry_repository(id),
				FOREIGN KEY (manifest_digest) REFERENCES container_registry_index(digest)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
			CREATE TABLE container_registry_layer_blob (
				digest TEXT PRIMARY KEY,
				size BIGINT NOT NULL
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
			CREATE TABLE container_registry_layer_manifest (
				manifest_digest TEXT NOT NULL,
				layer_blob_digest TEXT NOT NULL,

				FOREIGN KEY (manifest_digest) REFERENCES container_registry_manifest(digest),
				FOREIGN KEY (layer_blob_digest) REFERENCES container_registry_layer_blob(digest)
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

	Ok(())
}
