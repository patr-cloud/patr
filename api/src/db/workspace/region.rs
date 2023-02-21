use std::net::IpAddr;

use api_models::{
	models::workspace::region::InfrastructureCloudProvider,
	utils::Uuid,
};

use crate::{
	models::deployment::DEFAULT_DEPLOYMENT_REGIONS,
	query,
	query_as,
	Database,
};

pub struct DeploymentRegion {
	pub id: Uuid,
	pub name: String,
	pub cloud_provider: InfrastructureCloudProvider,
	pub workspace_id: Option<Uuid>,
	pub ready: bool,
	pub cf_cert_id: Option<String>,
	pub kubernetes_cluster_url: Option<String>,
	pub kubernetes_auth_username: Option<String>,
	pub kubernetes_auth_token: Option<String>,
	pub kubernetes_ca_data: Option<String>,
	pub kubernetes_ingress_ip_addr: Option<IpAddr>,
	pub message_log: Option<String>,
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
		CREATE TABLE deployment_region(
			id UUID CONSTRAINT deployment_region_pk PRIMARY KEY,
			name TEXT NOT NULL,
			provider INFRASTRUCTURE_CLOUD_PROVIDER NOT NULL,
			workspace_id UUID CONSTRAINT deployment_region_fk_workspace_id
				REFERENCES workspace(id),
			ready BOOLEAN NOT NULL,
			kubernetes_cluster_url TEXT,
			kubernetes_auth_username TEXT,
			kubernetes_auth_token TEXT,
			kubernetes_ca_data TEXT,
			kubernetes_ingress_ip_addr INET,
			message_log TEXT,
			cf_cert_id TEXT,
			CONSTRAINT deployment_region_chk_ready_or_not CHECK(
				(
					workspace_id IS NOT NULL AND
					cf_cert_id IS NOT NULL AND (
						(
							ready = TRUE AND
							kubernetes_cluster_url IS NOT NULL AND
							kubernetes_ca_data IS NOT NULL AND
							kubernetes_auth_username IS NOT NULL AND
							kubernetes_auth_token IS NOT NULL AND
							kubernetes_ingress_ip_addr IS NOT NULL
						) OR (
							ready = FALSE AND
							kubernetes_cluster_url IS NULL AND
							kubernetes_ca_data IS NULL AND
							kubernetes_auth_username IS NULL AND
							kubernetes_auth_username IS NULL AND
							kubernetes_ingress_ip_addr IS NULL
						)
					)
				) OR (
					workspace_id IS NULL AND
					cf_cert_id IS NULL AND
					kubernetes_cluster_url IS NULL AND
					kubernetes_ca_data IS NULL AND
					kubernetes_auth_username IS NULL AND
					kubernetes_auth_token IS NULL AND
					kubernetes_ingress_ip_addr IS NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// todo: create unique index
	// query!(
	// 	r#"
	// 	CREATE UNIQUE INDEX deployment_region_uq_workspace_id_name
	// 	ON deployment_region(workspace_id, name)
	// 	WHERE
	// 		deleted IS NULL AND
	// 		workspace_id IS NOT NULL;
	// 	"#
	// )
	// .execute(&mut *connection)
	// .await?;

	Ok(())
}

pub async fn initialize_region_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	for region in &DEFAULT_DEPLOYMENT_REGIONS {
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
	}

	Ok(())
}

pub async fn get_default_region_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	query!(
		r#"
		SELECT
			id as "id: Uuid"
		FROM
			deployment_region
		WHERE
			name = 'Singapore'
			AND provider = 'digitalocean'
			AND workspace_id IS NULL
			AND ready = TRUE;
		"#
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.id)
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
			cf_cert_id,
			message_log,
			kubernetes_cluster_url,
			kubernetes_auth_username,
			kubernetes_auth_token,
			kubernetes_ca_data,
			kubernetes_ingress_ip_addr as "kubernetes_ingress_ip_addr: _"
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
			cf_cert_id,
			message_log,
			kubernetes_cluster_url,
			kubernetes_auth_username,
			kubernetes_auth_token,
			kubernetes_ca_data,
			kubernetes_ingress_ip_addr as "kubernetes_ingress_ip_addr: _"
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
	cf_cert_id: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_region(
				id,
				name,
				provider,
				workspace_id,
				cf_cert_id,
				ready,
				kubernetes_cluster_url,
				kubernetes_auth_username,
				kubernetes_auth_token,
				kubernetes_ca_data,
				message_log
			)
		VALUES
			($1, $2, $3, $4, $5, FALSE, NULL, NULL, NULL, NULL, NULL);
		"#,
		region_id as _,
		name,
		cloud_provider as _,
		workspace_id as _,
		cf_cert_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn mark_deployment_region_as_ready(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	kubernetes_cluster_url: &str,
	kubernetes_auth_username: &str,
	kubernetes_auth_token: &str,
	kubernetes_ca_data: &str,
	kubernetes_ingress_ip_addr: &IpAddr,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			ready = TRUE,
			kubernetes_cluster_url = $2,
			kubernetes_auth_username = $3,
			kubernetes_auth_token = $4,
			kubernetes_ca_data = $5,
			kubernetes_ingress_ip_addr = $6
		WHERE
			id = $1;
		"#,
		region_id as _,
		kubernetes_cluster_url,
		kubernetes_auth_username,
		kubernetes_auth_token,
		kubernetes_ca_data,
		kubernetes_ingress_ip_addr as _
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
