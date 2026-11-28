use crate::prelude::*;

/// Initializes the object finalizer tables.
#[instrument(skip(connection))]
pub async fn initialize_object_finalizer_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up object finalizer tables");

	sqlx::query(
		r#"
		CREATE TABLE object_finalizer(
			object_id TEXT NOT NULL,
			deployment_id TEXT NOT NULL,
			finalizer TEXT NOT NULL,
			state TEXT NOT NULL,
			attempts INTEGER NOT NULL,
			last_error TEXT,
			last_updated DATETIME NOT NULL,

			PRIMARY KEY(object_id, finalizer),

			CONSTRAINT object_finalizer_chk_finalizer_enum CHECK(
				finalizer IN (
					'volume'
				)
			),

			CONSTRAINT object_finalizer_chk_state_enum CHECK(
				state IN (
					'pending',
					'running',
					'done',
					'failed'
				)
			),

			CONSTRAINT object_finalizer_chk_last_error_enum CHECK(
				last_error IN (
					'api_unreachable',
					'still_on_this_runner'
				)
			)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
