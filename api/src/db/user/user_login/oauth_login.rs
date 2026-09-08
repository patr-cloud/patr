use crate::prelude::*;

/// Initializes the OAuth login tables
#[instrument(skip(connection))]
pub async fn initialize_oauth_login_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up OAuth login tables");

	// A login by a 3rd party client, acting on behalf of a user
	query!(
		r#"
		CREATE TABLE oauth_login(
			login_id UUID NOT NULL,
			user_id UUID NOT NULL,
			client_id TEXT NOT NULL,
			scope TEXT NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			last_used TIMESTAMPTZ NOT NULL,
			revoked TIMESTAMPTZ,
			created_ip INET NOT NULL,
			created_user_agent TEXT NOT NULL,
			login_type USER_LOGIN_TYPE NOT NULL
				GENERATED ALWAYS AS ('oauth_login') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Consumed rows are kept: replaying one is how a stolen token is detected.
	query!(
		r#"
		CREATE TABLE oauth_refresh_token(
			id UUID NOT NULL,
			login_id UUID NOT NULL,
			token_hash TEXT NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			expiry TIMESTAMPTZ NOT NULL,
			consumed TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the OAuth login indices
#[instrument(skip(connection))]
pub async fn initialize_oauth_login_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up OAuth login indices");

	query!(
		r#"
		ALTER TABLE oauth_login
			ADD CONSTRAINT oauth_login_pk PRIMARY KEY(login_id),
			ADD CONSTRAINT oauth_login_uq_login_id_user_id UNIQUE(login_id, user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// For the authorized-apps screen.
	query!(
		r#"
		CREATE INDEX
			oauth_login_idx_user_id_client_id
		ON
			oauth_login(user_id, client_id)
		WHERE
			revoked IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE oauth_refresh_token
			ADD CONSTRAINT oauth_refresh_token_pk PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// One live refresh token per login. The old row has to be marked consumed
	// before the new one is inserted.
	query!(
		r#"
		CREATE UNIQUE INDEX
			oauth_refresh_token_uq_login_id
		ON
			oauth_refresh_token(login_id)
		WHERE
			consumed IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			oauth_refresh_token_idx_login_id
		ON
			oauth_refresh_token(login_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the OAuth login constraints
#[instrument(skip(connection))]
pub async fn initialize_oauth_login_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up OAuth login constraints");

	// The composite FK rejects an oauth_login whose parent is a web_login.
	query!(
		r#"
		ALTER TABLE oauth_login
			ADD CONSTRAINT oauth_login_fk
				FOREIGN KEY(
					login_id,
					user_id,
					login_type
				) REFERENCES user_login(
					login_id,
					user_id,
					login_type
				),
			ADD CONSTRAINT oauth_login_fk_client_id
				FOREIGN KEY(client_id) REFERENCES oauth_client(client_id),
			ADD CONSTRAINT oauth_login_chk_scope_not_empty CHECK(scope != '');
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE oauth_refresh_token
			ADD CONSTRAINT oauth_refresh_token_fk_login_id
				FOREIGN KEY(login_id) REFERENCES oauth_login(login_id),
			ADD CONSTRAINT oauth_refresh_token_chk_created_before_expiry
				CHECK(created < expiry);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
