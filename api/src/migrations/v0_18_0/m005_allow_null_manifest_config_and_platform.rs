//! Allows `config_blob_digest` and `platform` to be NULL on
//! `container_registry_manifest`. Index manifests (manifest lists — what
//! docker 29's containerd path pushes even for single-arch images) reference
//! other manifests and have no config blob or single platform of their own;
//! the PUT-manifest handler inserts NULL for both, which the old NOT NULL
//! constraints rejected — every index push failed with a 500.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		ALTER TABLE container_registry_manifest
			ALTER COLUMN config_blob_digest DROP NOT NULL,
			ALTER COLUMN platform DROP NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
