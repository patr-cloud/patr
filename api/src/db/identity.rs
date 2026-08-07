use crate::prelude::*;

/// Initializes the identity tables.
///
/// `identity` is the supertype of `"user"` and `service_account` — the two
/// things that can hold credentials, own workspace memberships and author
/// audit entries. Anything that needs to say "whoever did this" references
/// this table rather than picking one of the two concrete tables.
#[instrument(skip(connection))]
pub async fn initialize_identity_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up identity tables");
	query!(
		r#"
		CREATE TYPE IDENTITY_TYPE AS ENUM(
			'user',
			'service_account'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE identity(
			id UUID NOT NULL,
			type IDENTITY_TYPE NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the identity indices
#[instrument(skip(connection))]
pub async fn initialize_identity_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up identity indices");
	query!(
		r#"
		ALTER TABLE identity
			ADD CONSTRAINT identity_pk PRIMARY KEY(id),
			ADD CONSTRAINT identity_uq_id_type UNIQUE(id, type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the identity constraints.
///
/// There are none of its own — the subtypes point up at it, not the other way
/// round, so the constraints live on `"user"` and `service_account`.
#[instrument(skip(_connection))]
pub async fn initialize_identity_constraints(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up identity constraints");

	Ok(())
}
