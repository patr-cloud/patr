/// All API token related data of a user
mod api_token;
/// All web login related data of a user. Any login that is done through the
/// web dashboard will be stored here.
mod web_login;

use crate::prelude::*;

/// Initializes the user login tables
#[instrument(skip(connection))]
pub async fn initialize_user_login_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user login tables");
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

	// The registry of credentials that can act and leave an audit trail.
	// `user_login` registers into it (reusing login_id as the registry id);
	// future credential kinds (service account tokens, OAuth app tokens)
	// register alongside without audit_log ever growing per-kind columns.
	query!(
		r#"
		CREATE TYPE ACTOR_CLIENT_TYPE AS ENUM(
			'user_login'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE actor_client(
			id UUID NOT NULL,
			client_type ACTOR_CLIENT_TYPE NOT NULL
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
			created TIMESTAMPTZ NOT NULL,
			client_type ACTOR_CLIENT_TYPE NOT NULL
				GENERATED ALWAYS AS ('user_login') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	web_login::initialize_web_login_tables(&mut *connection).await?;
	api_token::initialize_api_token_tables(&mut *connection).await?;

	Ok(())
}

/// Initializes the user login indices
#[instrument(skip(connection))]
pub async fn initialize_user_login_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user login indices");
	query!(
		r#"
		ALTER TABLE actor_client
			ADD CONSTRAINT actor_client_pk PRIMARY KEY(id),
			ADD CONSTRAINT actor_client_uq_id_client_type UNIQUE(id, client_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_login
			ADD CONSTRAINT user_login_pk PRIMARY KEY(login_id),
			ADD CONSTRAINT user_login_uq_login_id_user_id UNIQUE(login_id, user_id),
			ADD CONSTRAINT user_login_uq_login_id_user_id_login_type UNIQUE(
				login_id, user_id, login_type
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	web_login::initialize_web_login_indices(&mut *connection).await?;
	api_token::initialize_api_token_indices(&mut *connection).await?;

	Ok(())
}

/// Initializes the user login constraints
#[instrument(skip(connection))]
pub async fn initialize_user_login_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user login constraints");
	query!(
		r#"
		ALTER TABLE user_login
			ADD CONSTRAINT user_login_fk_user_id
				FOREIGN KEY(user_id) REFERENCES "user"(id),
			ADD CONSTRAINT user_login_fk_login_id_client_type
				FOREIGN KEY(login_id, client_type)
					REFERENCES actor_client(id, client_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	web_login::initialize_web_login_constraints(&mut *connection).await?;
	api_token::initialize_api_token_constraints(&mut *connection).await?;

	query!(
		r#"
		CREATE FUNCTION GENERATE_LOGIN_ID() RETURNS UUID AS $$
		DECLARE
			id UUID;
		BEGIN
			id := gen_random_uuid();
			WHILE EXISTS(
				SELECT
					1
				FROM
					user_login
				WHERE
					login_id = id
			) LOOP
				id := gen_random_uuid();
			END LOOP;
			RETURN id;
		END;
		$$ LANGUAGE plpgsql;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
