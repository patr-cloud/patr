use crate::prelude::*;

/// Initializes the actor client tables
#[instrument(skip(connection))]
pub async fn initialize_actor_client_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up actor client tables");
	query!(
		r#"
		CREATE TYPE ACTOR_CLIENT_TYPE AS ENUM(
			'user_login',
			'service_account',
			'runner'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE actor_client(
			id UUID NOT NULL,
			actor_client_type ACTOR_CLIENT_TYPE NOT NULL,
			actor_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the actor client indices
#[instrument(skip(connection))]
pub async fn initialize_actor_client_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up actor client indices");
	query!(
		r#"
		ALTER TABLE actor_client
			ADD CONSTRAINT actor_client_pk PRIMARY KEY(id),
			ADD CONSTRAINT actor_client_uq_id_actor_client_type_actor_id
				UNIQUE(id, actor_client_type, actor_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the actor client constraints
#[instrument(skip(connection))]
pub async fn initialize_actor_client_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up actor client constraints");
	query!(
		r#"
		ALTER TABLE actor_client
			ADD CONSTRAINT actor_client_fk_actor_id
				FOREIGN KEY(actor_id) REFERENCES actor(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
