use crate::prelude::*;

/// Initializes the web login tables
#[instrument(skip(connection))]
pub async fn initialize_web_login_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE web_login(
			login_id UUID NOT NULL,
			user_id UUID NOT NULL,

			refresh_token TEXT NOT NULL,
			token_expiry TIMESTAMPTZ NOT NULL,

			created TIMESTAMPTZ NOT NULL,
			created_ip INET NOT NULL,
			created_location GEOMETRY NOT NULL,
			created_country TEXT NOT NULL,
			created_region TEXT NOT NULL,
			created_city TEXT NOT NULL,
			created_timezone TEXT NOT NULL,

			last_login TIMESTAMPTZ NOT NULL,
			last_activity TIMESTAMPTZ NOT NULL,
			last_activity_ip INET NOT NULL,
			last_activity_location GEOMETRY NOT NULL,
			last_activity_country TEXT NOT NULL,
			last_activity_region TEXT NOT NULL,
			last_activity_city TEXT NOT NULL,
			last_activity_timezone TEXT NOT NULL,
			last_activity_user_agent TEXT NOT NULL,

			login_type USER_LOGIN_TYPE NOT NULL GENERATED ALWAYS AS ('web_login') STORED
		);
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
	query!(
		r#"
		ALTER TABLE web_login
		ADD CONSTRAINT web_login_fk FOREIGN KEY(login_id, user_id, login_type)
		REFERENCES user_login(login_id, user_id, login_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			web_login_idx_user_id
		ON
			web_login(user_id);
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
	Ok(())
}
