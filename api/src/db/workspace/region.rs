use api_models::{
	models::workspace::region::InfrastructureCloudProvider,
	utils::Uuid,
};

use crate::{
	models::deployment::{DefaultDeploymentRegion, DEFAULT_DEPLOYMENT_REGIONS},
	query,
	query_as,
	Database,
};

pub struct DeploymentRegion {
	pub id: Uuid,
	pub name: String,
	pub cloud_provider: InfrastructureCloudProvider,
	pub ready: bool,
	pub workspace_id: Option<Uuid>,
	pub message_log: String,
}

pub async fn initialize_region_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
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
		CREATE TABLE deployment_region(
			id UUID CONSTRAINT deployment_region_pk PRIMARY KEY,
			name TEXT NOT NULL,
			provider INFRASTRUCTURE_CLOUD_PROVIDER NOT NULL,
			workspace_id UUID CONSTRAINT deployment_region_fk_workspace_id
				REFERENCES workspace(id),
			ready BOOLEAN NOT NULL,
			message_log TEXT NOT NULL,
			kubernetes_ca_data TEXT,
			kubernetes_auth_username TEXT,
			kubernetes_auth_token TEXT,
			CONSTRAINT deployment_region_chk_ready_or_not CHECK(
				(
					ready = TRUE AND
					kubernetes_ca_data IS NOT NULL AND
					kubernetes_auth_username IS NOT NULL AND
					kubernetes_auth_token IS NOT NULL
				) OR (
					ready = FALSE AND
					kubernetes_ca_data IS NULL AND
					kubernetes_auth_username IS NULL AND
					kubernetes_auth_token IS NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_region_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	for region in &DEFAULT_DEPLOYMENT_REGIONS {
		populate_region(&mut *connection, region).await?;
	}

	Ok(())
}

async fn populate_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	region: &DefaultDeploymentRegion,
) -> Result<Uuid, sqlx::Error> {
	let region_id = loop {
		let region_id = Uuid::new_v4();

		let row = query!(
			r#"
			SELECT
				id as "id: Uuid"
			FROM
				deployment_region
			WHERE
				id = $1;
			"#,
			region_id as _
		)
		.fetch_optional(&mut *connection)
		.await?;

		if row.is_none() {
			break region_id;
		}
	};

	// Populate leaf node
	query!(
		r#"
		INSERT INTO
			deployment_region(
				id,
				name,
				provider
			)
		VALUES
			($1, $2, $3);
		"#,
		region_id as _,
		region.name,
		region.cloud_provider as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(region_id)
}

pub async fn get_region_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<Option<DeploymentRegion>, sqlx::Error> {
	query_as!(
		DeploymentRegion,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			ready,
			workspace_id as "workspace_id: _",
			message_log
		FROM
			deployment_region
		WHERE
			id = $1;
		"#,
		region_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_all_deployment_regions_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<DeploymentRegion>, sqlx::Error> {
	query_as!(
		DeploymentRegion,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			ready,
			workspace_id as "workspace_id: _",
			message_log
		FROM
			deployment_region
		WHERE
			workspace_id IS NULL OR
			workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn add_deployment_region_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	name: &str,
	cloud_provider: &InfrastructureCloudProvider,
	workspace_id: &Uuid,
	kubernetes_ca_data: &str,
	kubernetes_auth_username: &str,
	kubernetes_auth_token: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_region(
				id,
				name,
				provider,
				workspace_id,
				ready,
				message_log,
				kubernetes_ca_data,
				kubernetes_auth_username,
				kubernetes_auth_token
			)
		VALUES
			($1, $2, $3, $4, FALSE, '', $5, $6, $7);
		"#,
		region_id as _,
		name,
		cloud_provider as _,
		workspace_id as _,
		kubernetes_ca_data,
		kubernetes_auth_username,
		kubernetes_auth_token
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
