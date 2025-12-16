use crate::prelude::*;

/// Initializes the oauth client tables
#[instrument(skip(connection))]
pub async fn initialize_client(connection: &mut DatabaseConnection) -> Result<(), sqlx::Error> {
	info!("Setting up oauth client tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS oauth_clients (
			id UUID PRIMARY KEY,
			name TEXT NOT NULL,
			secret TEXT NOT NULL,
			redirect_uri TEXT NOT NULL,
			created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
		 );
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
