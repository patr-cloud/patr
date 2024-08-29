use crate::prelude::*;

/// Initializes all container registry related tables
#[instrument(skip(connection))]
pub async fn initialize_container_registry_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up container registry tables");
	query!(
		r#"
		CREATE TABLE container_registry_repository_blob(
			blob_digest TEXT NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			size BIGINT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_manifest(
			manifest_digest TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_manifest_blob(
			manifest_digest TEXT NOT NULL,
			blob_digest TEXT NOT NULL,
			parent_blob_digest TEXT
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_repository(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			name CITEXT NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_repository_manifest(
			repository_id UUID NOT NULL,
			manifest_digest TEXT NOT NULL,
			architecture TEXT NOT NULL,
			os TEXT NOT NULL,
			variant TEXT NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE container_registry_repository_tag(
			repository_id UUID NOT NULL,
			tag TEXT NOT NULL,
			manifest_digest TEXT NOT NULL,
			last_updated TIMESTAMPTZ NOT NULL
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
	query!(
		r#"
		ALTER TABLE container_registry_repository_blob
		ADD CONSTRAINT container_registry_repository_blob_pk
		PRIMARY KEY(blob_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_manifest
		ADD CONSTRAINT container_registry_manifest_pk
		PRIMARY KEY(manifest_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_manifest_blob
		ADD CONSTRAINT container_registry_manifest_blob_pk
		PRIMARY KEY(manifest_digest, blob_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository
			ADD CONSTRAINT container_registry_repository_pk PRIMARY KEY(id),
			ADD CONSTRAINT container_registry_repository_uq_id_workspace_id UNIQUE(
				id, workspace_id
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			container_registry_repository_uq_workspace_id_name
		ON
			container_registry_repository(workspace_id, name)
		WHERE
			deleted IS NULL;
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
		ALTER TABLE container_registry_repository_tag
		ADD CONSTRAINT container_registry_repository_tag_pk
		PRIMARY KEY(repository_id, tag);
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
		ALTER TABLE container_registry_repository_blob
		ADD CONSTRAINT container_registry_repository_blob_chk_size_positive CHECK(
			size > 0
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_manifest_blob
			ADD CONSTRAINT container_registry_manifest_blob_fk_manifest_digest
				FOREIGN KEY(manifest_digest)
					REFERENCES container_registry_manifest(manifest_digest),
			ADD CONSTRAINT container_registry_manifest_blob_fk_blob_digest
				FOREIGN KEY(blob_digest) REFERENCES container_registry_repository_blob(blob_digest),
			ADD CONSTRAINT container_registry_manifest_blob_fk_parent_blob_digest
				FOREIGN KEY(parent_blob_digest)
					REFERENCES container_registry_repository_blob(blob_digest);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository
			ADD CONSTRAINT container_registry_repository_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT container_registry_repository_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted) 
					REFERENCES resource(id, owner_id, deleted)
					DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository_manifest
			ADD CONSTRAINT container_registry_repository_manifest_fk_manifest_digest
				FOREIGN KEY(manifest_digest)
					REFERENCES container_registry_manifest(manifest_digest),
			ADD CONSTRAINT container_registry_repository_manifest_fk_repository_id
				FOREIGN KEY(repository_id) REFERENCES container_registry_repository(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE container_registry_repository_tag
			ADD CONSTRAINT container_registry_repository_tag_fk_repository_id
				FOREIGN KEY(repository_id) REFERENCES container_registry_repository(id),
			ADD CONSTRAINT container_registry_repository_tag_fk_repo_id_manifest_digest
				FOREIGN KEY(repository_id, manifest_digest) REFERENCES
					container_registry_repository_manifest(repository_id, manifest_digest);
	"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
