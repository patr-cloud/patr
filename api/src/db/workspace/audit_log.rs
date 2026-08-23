use crate::prelude::*;

/// Initializes all audit log-related tables
#[instrument(skip(connection))]
pub async fn initialize_workspace_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up audit logs tables");

	query!(
		r#"
		CREATE TYPE AUDIT_LOG_TYPE AS ENUM (
			'create',
			'update',
			'delete'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE audit_log(
			id UUID NOT NULL,

			/* Where the action was performed from */
			timestamp TIMESTAMPTZ NOT NULL,
			ip INET NOT NULL,
			location GEOMETRY NOT NULL,
			user_agent TEXT NOT NULL,
			country TEXT NOT NULL,
			region TEXT NOT NULL,
			city TEXT NOT NULL,
			timezone TEXT NOT NULL,

			actor_client_id UUID NOT NULL,
			action AUDIT_LOG_TYPE NOT NULL,
			/* workspace_id is kept in case the resource is moved to another workspace */
			workspace_id UUID NOT NULL,
			resource_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes all audit log-related indices
#[instrument(skip(_connection))]
pub async fn initialize_workspace_indices(
	_connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up audit logs indices");

	Ok(())
}

/// Initializes all audit log-related constraints
#[instrument(skip(connection))]
pub async fn initialize_workspace_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up audit logs constraints");

	query!(
		r#"
		ALTER TABLE audit_log
			ADD CONSTRAINT audit_log_pkey PRIMARY KEY(id),
			ADD CONSTRAINT audit_log_workspace_id_fkey
				FOREIGN KEY(workspace_id)
					REFERENCES workspace(id),
			ADD CONSTRAINT audit_log_resource_id_fkey
				FOREIGN KEY(resource_id)
					REFERENCES resource(id),
			ADD CONSTRAINT audit_log_fk_actor_client_id
				FOREIGN KEY(actor_client_id)
					REFERENCES actor_client(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE FUNCTION GENERATE_AUDIT_LOG_ID() RETURNS UUID AS $$
		DECLARE
			audit_log_id UUID;
		BEGIN
			audit_log_id := gen_random_uuid();
			WHILE EXISTS(
				SELECT
					1
				FROM
					audit_log
				WHERE
					id = audit_log_id
			) LOOP
				audit_log_id := gen_random_uuid();
			END LOOP;
			RETURN audit_log_id;
		END;
		$$ LANGUAGE plpgsql;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
