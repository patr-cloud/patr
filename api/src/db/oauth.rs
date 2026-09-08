use crate::prelude::*;

/// Initializes the OAuth tables
#[instrument(skip(connection))]
pub async fn initialize_oauth_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up OAuth tables");

	// Display metadata for the clients in the config, upserted at boot.
	// Secrets, redirect URIs and scopes are only ever read from the config.
	// Removing a client is deleting its row.
	query!(
		r#"
		CREATE TABLE oauth_client(
			client_id TEXT NOT NULL,
			name TEXT NOT NULL,
			logo_url TEXT NOT NULL,
			client_uri TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the OAuth indices
#[instrument(skip(connection))]
pub async fn initialize_oauth_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up OAuth indices");

	query!(
		r#"
		ALTER TABLE oauth_client
			ADD CONSTRAINT oauth_client_pk PRIMARY KEY(client_id),
			ADD CONSTRAINT oauth_client_chk_client_id_not_empty CHECK(client_id != '');
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the OAuth constraints
#[instrument(skip(_connection))]
pub async fn initialize_oauth_constraints(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up OAuth constraints");
	Ok(())
}
