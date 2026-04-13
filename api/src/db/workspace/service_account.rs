use crate::prelude::*;

/// Initializes the service account tables.
///
/// A service account's id is the same id in three registries: `resource` (it
/// is permission-gated like anything else), `workspace_actor` (role bindings
/// are granted on it) and `actor_client` (it authenticates and leaves an
/// audit trail). It is workspace-scoped by construction, so its actor is
/// one-to-one with the account and needs no membership row — unlike a user,
/// whose actor is per-workspace.
#[instrument(skip(connection))]
pub async fn initialize_service_account_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up service account tables");
	query!(
		r#"
		CREATE TABLE service_account(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			name VARCHAR(100) NOT NULL,
			description TEXT,
			token_hash TEXT NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			deleted TIMESTAMPTZ,
			actor_type WORKSPACE_ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('service_account') STORED,
			actor_client_type ACTOR_CLIENT_TYPE NOT NULL
				GENERATED ALWAYS AS ('service_account') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the service account indices
#[instrument(skip(connection))]
pub async fn initialize_service_account_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up service account indices");
	query!(
		r#"
		ALTER TABLE service_account
			ADD CONSTRAINT service_account_pk
				PRIMARY KEY(id),
			ADD CONSTRAINT service_account_uq_id_workspace_id
				UNIQUE(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			service_account_uq_workspace_id_name
		ON
			service_account(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the service account constraints
#[instrument(skip(connection))]
pub async fn initialize_service_account_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up service account constraints");
	query!(
		r#"
		ALTER TABLE service_account
			ADD CONSTRAINT service_account_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT service_account_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, workspace_id, deleted),
			ADD CONSTRAINT service_account_fk_id_actor_type
				FOREIGN KEY(id, actor_type)
					REFERENCES workspace_actor(id, actor_type),
			ADD CONSTRAINT service_account_fk_id_actor_workspace_id
				FOREIGN KEY(id, workspace_id)
					REFERENCES workspace_actor(id, workspace_id),
			ADD CONSTRAINT service_account_fk_id_actor_client_type
				FOREIGN KEY(id, actor_client_type)
					REFERENCES actor_client(id, actor_client_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
