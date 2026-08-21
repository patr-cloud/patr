use crate::prelude::*;

/// Initializes the user tables
#[instrument(skip(connection))]
pub async fn initialize_user_data_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user tables");
	query!(
		r#"
		CREATE TABLE "user"(
			id UUID NOT NULL,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			email CITEXT NOT NULL,
			workspace_limit INTEGER NOT NULL,
			password_reset_token TEXT,
			password_reset_token_expiry TIMESTAMPTZ NULL,
			password_reset_attempts INT NULL,
			mfa_secret TEXT
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the user data indices
#[instrument(skip(connection))]
pub async fn initialize_user_data_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user data indices");
	query!(
		r#"
		ALTER TABLE "user"
			ADD CONSTRAINT user_pk PRIMARY KEY(id),
			ADD CONSTRAINT user_uq_email UNIQUE(email);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			user_idx_created
		ON
			"user"
		(created);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the user data constraints. There are none.
pub async fn initialize_user_data_constraints(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	Ok(())
}
