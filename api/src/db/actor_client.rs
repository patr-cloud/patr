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
			actor_client_type ACTOR_CLIENT_TYPE NOT NULL
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
			ADD CONSTRAINT actor_client_uq_id_actor_client_type UNIQUE(id, actor_client_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the actor client constraints
#[instrument(skip(_connection))]
pub async fn initialize_actor_client_constraints(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up actor client constraints");
	Ok(())
}
