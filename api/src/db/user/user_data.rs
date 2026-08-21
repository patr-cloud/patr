use crate::prelude::*;

/// Initializes the user tables
#[instrument(skip(connection))]
pub async fn initialize_user_data_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user tables");
	query!(
		r#"
		/*
		 * `email` sits where `recovery_email` used to, because m011 gets here
		 * by renaming that column rather than adding a new one. Keeping the
		 * position identical means a freshly initialized schema matches a
		 * migrated one — which matters because `SELECT "user".*` decodes
		 * positionally against the offline sqlx cache, and a column-order
		 * difference between the two would blow up at runtime on whichever
		 * one the cache wasn't built from.
		 */
		CREATE TABLE "user"(
			id UUID NOT NULL,
			password TEXT NOT NULL,
			first_name VARCHAR(100) NOT NULL,
			last_name VARCHAR(100) NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			/*
			 * A user's email is their unique identifier. CITEXT so that
			 * comparison and the UNIQUE constraint are both case-insensitive
			 * without every call site having to remember to lowercase.
			 */
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

/// Initializes the user data constraints.
///
/// There are none: the only invariants on `"user"` are its primary key and
/// the unique email, both of which are set up alongside the indices. The
/// email doesn't need a lower-case CHECK because the column is `CITEXT` —
/// case-insensitivity is a property of the type, not something every writer
/// has to remember.
pub async fn initialize_user_data_constraints(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	Ok(())
}
