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
		CREATE TYPE CREDENTIAL_TYPE AS ENUM(
			'web_login',
			'api_token',
			'service_account'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE credential(
			credential_id UUID NOT NULL,
			identity_id UUID NOT NULL,
			type CREDENTIAL_TYPE NOT NULL,
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

/// Initializes the user login indices
#[instrument(skip(connection))]
pub async fn initialize_user_login_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user login indices");
	query!(
		r#"
		ALTER TABLE credential
			ADD CONSTRAINT credential_pk PRIMARY KEY(credential_id),
			ADD CONSTRAINT credential_uq_credential_id_identity_id UNIQUE(
				credential_id, identity_id
			),
			ADD CONSTRAINT credential_uq_credential_id_identity_id_type UNIQUE(
				credential_id, identity_id, type
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Web sessions and API tokens are many-per-identity; a service account
	// holds exactly one non-rotating credential.
	query!(
		r#"
		CREATE UNIQUE INDEX
			credential_uq_identity_id_service_account
		ON
			credential(identity_id)
		WHERE
			type = 'service_account';
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
		ALTER TABLE credential
		ADD CONSTRAINT credential_fk_identity_id
		FOREIGN KEY(identity_id) REFERENCES identity(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	web_login::initialize_web_login_constraints(&mut *connection).await?;
	api_token::initialize_api_token_constraints(&mut *connection).await?;

	query!(
		r#"
		CREATE FUNCTION GENERATE_CREDENTIAL_ID() RETURNS UUID AS $$
		DECLARE
			id UUID;
		BEGIN
			id := gen_random_uuid();
			WHILE EXISTS(
				SELECT
					1
				FROM
					credential
				WHERE
					credential_id = id
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
