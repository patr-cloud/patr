use crate::prelude::*;

/// Initializes the region tables
#[instrument(skip(connection))]
pub async fn initialize_region_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up region tables");
	query!(
		r#"
		CREATE TYPE INFRASTRUCTURE_CLOUD_PROVIDER AS ENUM(
			'digitalocean',
			'other'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE REGION_STATUS AS ENUM(
			'creating',
			'active',
			'errored',
			'deleted',
			'disconnected',
			'coming_soon'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE region(
			id UUID NOT NULL,
			name TEXT NOT NULL,
			provider INFRASTRUCTURE_CLOUD_PROVIDER NOT NULL,
			workspace_id UUID,
			message_log TEXT,
			status REGION_STATUS NOT NULL,
			ingress_hostname TEXT,
			cloudflare_certificate_id TEXT,
			config_file JSON,
			deleted TIMESTAMPTZ,
			disconnected_at TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the region indices
#[instrument(skip(connection))]
pub async fn initialize_region_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up region indices");
	query!(
		r#"
		ALTER TABLE region
		ADD CONSTRAINT region_pk
		PRIMARY KEY(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			region_uq_workspace_id_name
		ON
			region(workspace_id, name)
		WHERE
			deleted IS NULL AND
			workspace_id IS NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the region constraints
#[instrument(skip(connection))]
pub async fn initialize_region_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up region constraints");
	query!(
		r#"
		ALTER TABLE region
			ADD CONSTRAINT region_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT region_chk_status CHECK(
				(
					status = 'creating'
				) OR (
					status = 'active' AND
					ingress_hostname IS NOT NULL AND
					cloudflare_certificate_id IS NOT NULL AND
					config_file IS NOT NULL AND
					disconnected_at IS NULL
				) OR (
					status = 'errored'
				) OR (
					status = 'deleted' AND
					deleted IS NOT NULL
				) OR (
					status = 'disconnected' AND
					ingress_hostname IS NOT NULL AND
					cloudflare_certificate_id IS NOT NULL AND
					config_file IS NOT NULL AND
					disconnected_at IS NOT NULL
				) OR (
					status = 'coming_soon'
				)
			),
			ADD CONSTRAINT region_fk_id_workspace_id
				FOREIGN KEY (id, workspace_id) 
					REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
