use crate::prelude::*;

/// Initializes the service account tables
#[instrument(skip(connection))]
pub async fn initialize_service_account_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up service account tables");
	query!(
		r#"
		CREATE TABLE service_account(
			id UUID NOT NULL,
			name VARCHAR(100) NOT NULL,
			workspace_id UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			description TEXT,
			token_hash TEXT NOT NULL,
			deleted TIMESTAMPTZ,
			identity_type IDENTITY_TYPE NOT NULL
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
			ADD CONSTRAINT service_account_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, owner_id, deleted),
			ADD CONSTRAINT service_account_fk_id_identity_type
				FOREIGN KEY(id, identity_type) REFERENCES identity(id, type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
