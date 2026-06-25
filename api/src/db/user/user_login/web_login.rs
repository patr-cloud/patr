use crate::prelude::*;

/// Initializes the web login tables
#[instrument(skip(connection))]
pub async fn initialize_web_login_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up web login tables");
	query!(
		r#"
		CREATE TABLE web_login(
			login_id UUID NOT NULL,
			original_login_id UUID, /* In case this login was magically swapped, what's the original one */
			user_id UUID NOT NULL,

			refresh_token TEXT NOT NULL,
			/* Expiry of the refresh token. Access tokens are independently
			 * gated by their own JWT `exp` claim — do NOT use this column to
			 * decide whether an access token is still valid. */
			token_expiry TIMESTAMPTZ NOT NULL,

			created TIMESTAMPTZ NOT NULL,
			created_ip INET NOT NULL,
			created_location GEOMETRY NOT NULL,
			created_user_agent TEXT NOT NULL,
			created_country TEXT NOT NULL,
			created_region TEXT NOT NULL,
			created_city TEXT NOT NULL,
			created_timezone TEXT NOT NULL,

			login_type USER_LOGIN_TYPE NOT NULL GENERATED ALWAYS AS ('web_login') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE SOCIAL_LOGIN_PROVIDER AS ENUM(
			'github'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_social_login(
			user_id     UUID                  NOT NULL,
			provider    SOCIAL_LOGIN_PROVIDER NOT NULL,
			external_id TEXT                  NOT NULL,
			linked_at   TIMESTAMPTZ           NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the web login indices
#[instrument(skip(connection))]
pub async fn initialize_web_login_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up web login indices");
	query!(
		r#"
		ALTER TABLE web_login
		ADD CONSTRAINT web_login_pk
		PRIMARY KEY(login_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			web_login_idx_login_id
		ON
			web_login(login_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_social_login
		ADD CONSTRAINT user_social_login_pk
		PRIMARY KEY (provider, external_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX user_social_login_idx_user_id
			ON user_social_login(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the web login constraints
#[instrument(skip(connection))]
pub async fn initialize_web_login_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up web login constraints");
	query!(
		r#"
		ALTER TABLE web_login
		ADD CONSTRAINT web_login_fk
		FOREIGN KEY(
			login_id,
			user_id,
			login_type
		) REFERENCES user_login(
			login_id,
			user_id,
			login_type
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_social_login
			ADD CONSTRAINT user_social_login_uq_user_provider
				UNIQUE (user_id, provider),
			ADD CONSTRAINT user_social_login_fk_user_id
				FOREIGN KEY (user_id) REFERENCES "user"(id),
			ADD CONSTRAINT user_social_login_chk_external_id_not_empty
				CHECK (external_id <> '');
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
