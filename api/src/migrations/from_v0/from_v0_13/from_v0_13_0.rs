use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	allow_private_docker_registry(connection, config).await?;

	Ok(())
}

async fn allow_private_docker_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment
			ADD COLUMN use_private_regcred BOOL NOT NULL DEFAULT FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
			DROP CONSTRAINT deployment_chk_repository_id_is_valid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
			ADD CONSTRAINT deployment_chk_repository_id_is_valid CHECK(
				(
					registry = 'registry.patr.cloud' AND
					use_private_regcred = false AND
					image_name IS NULL AND
					repository_id IS NOT NULL
				)
				OR (
					registry != 'registry.patr.cloud' AND
					image_name IS NOT NULL AND
					repository_id IS NULL
				)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
