use crate::prelude::*;

/// All tables related to the audit logs go here
mod audit_log;
/// The data stored in the container registry
mod container_registry;
/// The list of domains that are added to a workspace
mod domain;
/// The pending email invites to join a workspace
mod invite;

/// The list of deployments that are present in a workspace
mod deployment;
/// The list of databases that are created in a workspace
/// The list of Managed URLs that are created in a workspace
mod managed_url;
/// The list of static sites that are created in a workspace
/// The list of deployment volumes that are created in a workspace
mod volume;

/// The list of runners that are a part of a workspace
mod runner;
/// The list of secrets that are added to a workspace
mod secret;
/// The list of service accounts that belong to a workspace
mod service_account;

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

	audit_log::initialize_workspace_tables(connection).await?;
	container_registry::initialize_container_registry_tables(connection).await?;
	domain::initialize_domain_tables(connection).await?;
	invite::initialize_workspace_user_invite_tables(connection).await?;

	deployment::initialize_deployment_tables(connection).await?;
	managed_url::initialize_managed_url_tables(connection).await?;
	volume::initialize_volume_tables(connection).await?;

	runner::initialize_runner_tables(connection).await?;
	secret::initialize_secret_tables(connection).await?;
	service_account::initialize_service_account_tables(connection).await?;

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

	audit_log::initialize_workspace_indices(connection).await?;
	container_registry::initialize_container_registry_indices(connection).await?;
	domain::initialize_domain_indices(connection).await?;
	invite::initialize_workspace_user_invite_indices(connection).await?;

	deployment::initialize_deployment_indices(connection).await?;
	managed_url::initialize_managed_url_indices(connection).await?;
	volume::initialize_volume_indices(connection).await?;

	runner::initialize_runner_indices(connection).await?;
	secret::initialize_secret_indices(connection).await?;
	service_account::initialize_service_account_indices(connection).await?;

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

	audit_log::initialize_workspace_constraints(connection).await?;
	container_registry::initialize_container_registry_constraints(connection).await?;
	domain::initialize_domain_constraints(connection).await?;
	invite::initialize_workspace_user_invite_constraints(connection).await?;

	deployment::initialize_deployment_constraints(connection).await?;
	managed_url::initialize_managed_url_constraints(connection).await?;
	volume::initialize_volume_constraints(connection).await?;

	runner::initialize_runner_constraints(connection).await?;
	secret::initialize_secret_constraints(connection).await?;
	service_account::initialize_service_account_constraints(connection).await?;

	Ok(())
}
