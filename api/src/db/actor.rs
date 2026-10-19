use crate::prelude::*;

/// Initializes the actor tables.
///
/// An actor is the accountable "who": one system-wide unique id per user or
/// service account, shared with its row in the subtype table. Every
/// credential in `actor_client` names the actor that owns it, so a credential
/// can always be traced back to its owner. Actors are never deleted; the
/// subtype tables carry deletion.
#[instrument(skip(connection))]
pub async fn initialize_actor_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up actor tables");
	query!(
		r#"
		CREATE TYPE ACTOR_TYPE AS ENUM(
			'user',
			'service_account'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE actor(
			id UUID NOT NULL,
			actor_type ACTOR_TYPE NOT NULL,
			created TIMESTAMPTZ NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the actor indices
#[instrument(skip(connection))]
pub async fn initialize_actor_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up actor indices");
	query!(
		r#"
		ALTER TABLE actor
			ADD CONSTRAINT actor_pk PRIMARY KEY(id),
			ADD CONSTRAINT actor_uq_id_actor_type UNIQUE(id, actor_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the actor constraints
#[instrument(skip(_connection))]
pub async fn initialize_actor_constraints(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up actor constraints");
	Ok(())
}
