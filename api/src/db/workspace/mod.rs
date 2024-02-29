use crate::prelude::*;

/// The data stored in the container registry
mod container_registry;
/// The list of domains that are added to a workspace
mod domain;
/// All infrastructure related stuff goes here
mod infrastructure;
/// The list of regions that are added to a workspace
mod region;
/// The list of secrets that are added to a workspace
mod secret;

/// Initializes all workspace-related tables
#[instrument(skip(connection))]
pub async fn initialize_workspace_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace tables");
	query!(
		r#"
		CREATE TABLE workspace(
			id UUID NOT NULL,
			name CITEXT NOT NULL,
			super_admin_id UUID NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Ref: https://www.postgresql.org/docs/13/datatype-enum.html
	query!(
		r#"
		CREATE TYPE RESOURCE_OWNER_TYPE AS ENUM(
			'personal',
			'business'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	domain::initialize_domain_tables(connection).await?;
	container_registry::initialize_container_registry_tables(connection).await?;
	secret::initialize_secret_tables(connection).await?;
	region::initialize_region_tables(connection).await?;
	infrastructure::initialize_infrastructure_tables(connection).await?;

	Ok(())
}

/// Initializes all workspace-related indices
#[instrument(skip(connection))]
pub async fn initialize_workspace_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace indices");
	query!(
		r#"
		ALTER TABLE workspace
			ADD CONSTRAINT workspace_pk PRIMARY KEY(id),
			ADD CONSTRAINT workspace_uq_id_super_admin_id
				UNIQUE(id, super_admin_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_idx_super_admin_id
		ON
			workspace
		(super_admin_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			workspace_uq_name
		ON
			workspace(name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	domain::initialize_domain_indices(connection).await?;
	container_registry::initialize_container_registry_indices(connection).await?;
	secret::initialize_secret_indices(connection).await?;
	region::initialize_region_indices(connection).await?;
	infrastructure::initialize_infrastructure_indices(connection).await?;

	Ok(())
}

/// Initializes all workspace-related constraints
#[instrument(skip(connection))]
pub async fn initialize_workspace_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up workspace constraints");
	query!(
		r#"
		ALTER TABLE workspace
			ADD CONSTRAINT workspace_fk_id FOREIGN KEY(id) REFERENCES resource(id)
				DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT workspace_fk_super_admin_id
				FOREIGN KEY(super_admin_id) REFERENCES "user"(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	domain::initialize_domain_constraints(connection).await?;
	container_registry::initialize_container_registry_constraints(connection).await?;
	secret::initialize_secret_constraints(connection).await?;
	region::initialize_region_constraints(connection).await?;
	infrastructure::initialize_infrastructure_constraints(connection).await?;

	Ok(())
}
