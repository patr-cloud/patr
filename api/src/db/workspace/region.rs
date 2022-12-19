use std::net::IpAddr;

use api_models::{
	models::workspace::region::InfrastructureCloudProvider,
	utils::Uuid,
};
use chrono::{DateTime, Utc};

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
	pub message_log: Option<String>,
	pub config_file: Option<String>,
	pub kubernetes_ingress_ip_addr: Option<IpAddr>,
	pub is_disconnected: Option<bool>,
	pub last_disconnected: Option<DateTime<Utc>>,
}

pub struct BYOCRegion {
	pub id: Uuid,
	pub name: String,
	pub cloud_provider: InfrastructureCloudProvider,
	pub ready: bool,
	pub workspace_id: Uuid,
	pub config_file: String,
	pub is_disconnected: Option<bool>,
	pub last_disconnected: Option<DateTime<Utc>>,
}

impl DeploymentRegion {
	pub fn is_byoc_region(&self) -> bool {
		self.workspace_id.is_some()
	}
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
		CREATE TYPE REGION_STATUS AS ENUM(
			'created',
			'active',
			'deleted'
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
			config_file JSON,
			api_token TEXT,
			cluster_name TEXT,
			num_node INTEGER,
			node_name TEXT,
			node_size TEXT,
			do_cluster_id UUID,
			kubernetes_ingress_ip_addr INET,
			message_log TEXT,
			deleted TIMESTAMPTZ,
			status REGION_STATUS NOT NULL DEFAULT 'created',
			is_disconnected BOOLEAN,
			last_disconnected TIMESTAMPTZ,
			CONSTRAINT deployment_region_chk_ready_or_not CHECK(
				(
					workspace_id IS NOT NULL AND (
						(
							ready = TRUE AND
							config_file IS NOT NULL AND
							kubernetes_ingress_ip_addr IS NOT NULL
						) OR (
							ready = FALSE AND
							config_file IS NULL AND
							kubernetes_ingress_ip_addr IS NULL
						)
					)
				) OR (
					workspace_id IS NULL AND
					config_file IS NULL AND
					kubernetes_ingress_ip_addr IS NULL
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
				provider,
				ready
			)
		VALUES
			($1, $2, $3, $4);
		"#,
		region_id as _,
		region.name,
		region.cloud_provider as _,
		region.is_ready
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
			message_log,
			config_file as "config_file: _",
			kubernetes_ingress_ip_addr as "kubernetes_ingress_ip_addr: _",
			is_disconnected as "is_disconnected: _",
			last_disconnected as "last_disconnected: _"
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
			message_log,
			config_file as "config_file!: _",
			kubernetes_ingress_ip_addr as "kubernetes_ingress_ip_addr: _",
			is_disconnected as "is_disconnected: _",
			last_disconnected as "last_disconnected: _"
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

pub async fn get_all_ready_byoc_region(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<BYOCRegion>, sqlx::Error> {
	query_as!(
		BYOCRegion,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			ready,
			workspace_id as "workspace_id!: _",
			config_file as "config_file!: _",
			is_disconnected as "is_disconnected: _",
			last_disconnected as "last_disconnected: _"
		FROM
			deployment_region
		WHERE
			workspace_id IS NOT NULL AND
			ready = TRUE AND
			deleted IS NULL AND
			status != 'deleted';
		"#,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn update_byoc_region_connected(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	is_disconnected: bool,
	last_disconnected: Option<&DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			is_disconnected = $1,
			last_disconnected = $2 
		WHERE
			id = $3;
		"#,
		is_disconnected,
		last_disconnected as _,
		region_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_deployment_region_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	name: &str,
	cloud_provider: &InfrastructureCloudProvider,
	workspace_id: &Uuid,
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
				config_file,
				status
			)
		VALUES
			($1, $2, $3, $4, FALSE, NULL, 'created');
		"#,
		region_id as _,
		name,
		cloud_provider as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_do_deployment_region_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	region_id: &Uuid,
	name: &str,
	cloud_provider: &InfrastructureCloudProvider,
	api_token: &str,
	cluster_name: &str,
	num_node: &u16,
	node_name: &str,
	node_size: &str,
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
				config_file,
				api_token,
				cluster_name,
				num_node,
				node_name,
				node_size,
				status
			)
		VALUES
			($1, $2, $3, $4, FALSE, NULL, $5, $6, $7, $8, $9, 'created');
		"#,
		region_id as _,
		name,
		cloud_provider as _,
		workspace_id as _,
		api_token,
		cluster_name,
		*num_node as i16,
		node_name,
		node_size,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_do_cluster_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	cluster_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			do_cluster_id = $1
		WHERE
			id = $2;
		"#,
		cluster_id as _,
		region_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn mark_deployment_region_as_ready(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	kube_config: &serde_json::Value,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			ready = TRUE,
			status = 'active',
			config_file = $1
		WHERE
			id = $2;
		"#,
		kube_config as _,
		region_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn append_messge_log_for_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	message: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			message_log = CONCAT(message_log, $2::TEXT)
		WHERE
			id = $1;
		"#,
		region_id as _,
		message
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	deletion_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			deleted = $2,
			status = 'deleted',
			ready = FALSE
		WHERE
			id = $1;
		"#,
		region_id as _,
		deletion_time
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
