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
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE service_account_role(
			service_account_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			role_id UUID NOT NULL
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

	query!(
		r#"
		ALTER TABLE service_account_role
		ADD CONSTRAINT service_account_role_pk
		PRIMARY KEY(service_account_id, role_id);
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
					REFERENCES resource(id, owner_id, deleted);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE service_account_role
			ADD CONSTRAINT service_account_role_fk_service_account_id_workspace_id
				FOREIGN KEY(service_account_id, workspace_id)
					REFERENCES service_account(id, workspace_id),
			ADD CONSTRAINT service_account_role_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id)
					REFERENCES role(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
