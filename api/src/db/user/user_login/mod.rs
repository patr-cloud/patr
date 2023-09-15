mod api_token;
mod web_login;

pub use self::{api_token::*, web_login::*};
use crate::prelude::*;

/// Initializes the user login tables
#[instrument(skip(connection))]
pub async fn initialize_user_login_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TYPE USER_LOGIN_TYPE AS ENUM(
			'api_token',
			'web_login'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_login(
			login_id UUID NOT NULL,
			user_id UUID NOT NULL,
            login_type USER_LOGIN_TYPE NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	web_login::initialize_web_login_tables(&mut *connection).await?;
	api_token::initialize_api_token_tables(&mut *connection).await?;

	Ok(())
}

/// Initializes the user login constraints
#[instrument(skip(connection))]
pub async fn initialize_user_login_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE user_login
			ADD CONSTRAINT user_login_pk PRIMARY KEY(login_id),
			ADD CONSTRAINT user_login_fk_user_id
				FOREIGN KEY(user_id) REFERENCES "user"(id),
			ADD CONSTRAINT user_login_uq_login_id_user_id UNIQUE(login_id, user_id),
			ADD CONSTRAINT user_login_uq_login_id_user_id_login_type UNIQUE(
				login_id, user_id, login_type
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	web_login::initialize_web_login_constraints(&mut *connection).await?;
	api_token::initialize_api_token_constraints(&mut *connection).await?;

	Ok(())
}
