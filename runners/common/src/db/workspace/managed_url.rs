use crate::prelude::*;

/// Initializes the managed URL tables.
#[instrument(skip(connection))]
pub async fn initialize_managed_url_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up managed URL tables");

	query(
		r#"
		CREATE TABLE managed_url(
			id TEXT NOT NULL PRIMARY KEY,
			host TEXT NOT NULL,
			path TEXT NOT NULL,
			deployment_id TEXT NOT NULL,
			port INTEGER NOT NULL,

			CONSTRAINT managed_url_chk_port_range
				CHECK(port > 0 AND port <= 65535),
			CONSTRAINT managed_url_chk_host_nonempty
				CHECK(LENGTH(TRIM(host)) > 0),

			FOREIGN KEY(deployment_id) REFERENCES deployment(id)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the managed URL indices.
#[instrument(skip(connection))]
pub async fn initialize_managed_url_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up managed URL indices");

	query(
		r#"
		CREATE INDEX
			managed_url_idx_deployment_id
		ON
			managed_url(deployment_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
