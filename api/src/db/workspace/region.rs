use api_models::{
	models::workspace::region::{InfrastructureCloudProvider, RegionStatus},
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use kube::config::Kubeconfig;
use url::Host;

use crate::{
	models::deployment::DEFAULT_DEPLOYMENT_REGIONS,
	query,
	query_as,
	Database,
};

pub struct Region {
	pub id: Uuid,
	pub name: String,
	pub cloud_provider: InfrastructureCloudProvider,
	pub workspace_id: Option<Uuid>,
	pub ingress_hostname: Option<String>,
	pub message_log: Option<String>,
	pub config_file: Option<Kubeconfig>,
	pub status: RegionStatus,
	pub disconnected_at: Option<DateTime<Utc>>,
}

impl Region {
	pub fn is_byoc_region(&self) -> bool {
		self.workspace_id.is_some()
	}
}

struct DbRegion {
	pub id: Uuid,
	pub name: String,
	pub cloud_provider: InfrastructureCloudProvider,
	pub workspace_id: Option<Uuid>,
	pub ingress_hostname: Option<String>,
	pub message_log: Option<String>,
	pub config_file: Option<sqlx::types::Json<Kubeconfig>>,
	pub status: RegionStatus,
	pub disconnected_at: Option<DateTime<Utc>>,
}

impl From<DbRegion> for Region {
	fn from(from: DbRegion) -> Self {
		Self {
			id: from.id,
			name: from.name,
			cloud_provider: from.cloud_provider,
			workspace_id: from.workspace_id,
			ingress_hostname: from.ingress_hostname,
			message_log: from.message_log,
			config_file: from.config_file.map(|config| config.0),
			status: from.status,
			disconnected_at: from.disconnected_at,
		}
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
		CREATE TABLE deployment_region(
			id UUID CONSTRAINT deployment_region_pk PRIMARY KEY,
			name TEXT NOT NULL,
			provider INFRASTRUCTURE_CLOUD_PROVIDER NOT NULL,
			workspace_id UUID CONSTRAINT deployment_region_fk_workspace_id
				REFERENCES workspace(id),
			ingress_hostname TEXT,
			message_log TEXT,
			config_file JSON,
			deleted TIMESTAMPTZ,
			status REGION_STATUS NOT NULL,
			disconnected_at TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// todo: add check constraint
	// 	1. if the status is deleted, then the deleted should have timestamp
	//  2. if the status is disconnected, then the last disconnected should have
	// timestamp

	Ok(())
}

pub async fn initialize_region_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment_region
			ADD CONSTRAINT deployment_region_fk_id_workspace_id
			FOREIGN KEY (id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	for region in DEFAULT_DEPLOYMENT_REGIONS {
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

		query!(
			r#"
			INSERT INTO
				deployment_region(
					id,
					name,
					provider,
					status
				)
			VALUES
				($1, $2, $3, $4);
			"#,
			region_id as _,
			region.name,
			region.cloud_provider as _,
			region.status as _,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

pub async fn get_region_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<Option<Region>, sqlx::Error> {
	query_as!(
		DbRegion,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			deployment_region
		WHERE
			id = $1;
		"#,
		region_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|opt_region| opt_region.map(|region| region.into()))
}

pub async fn get_all_deployment_regions_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Region>, sqlx::Error> {
	query_as!(
		DbRegion,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			deployment_region
		WHERE
			status != 'deleted' AND
			(
				workspace_id IS NULL OR
				workspace_id = $1
			);
		"#,
		workspace_id as _,
	)
	.fetch(&mut *connection)
	.map_ok(|region| region.into())
	.try_collect()
	.await
}

pub async fn get_all_active_byoc_region(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<Region>, sqlx::Error> {
	query_as!(
		DbRegion,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			deployment_region
		WHERE
			workspace_id IS NOT NULL AND
			status = 'active';
		"#,
	)
	.fetch(&mut *connection)
	.map_ok(|region| region.into())
	.try_collect()
	.await
}

pub async fn get_all_disconnected_byoc_region(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<Region>, sqlx::Error> {
	query_as!(
		DbRegion,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			deployment_region
		WHERE
			workspace_id IS NOT NULL AND
			status = 'disconnected' AND
			disconnected_at IS NOT NULL
		ORDER BY disconnected_at;
		"#,
	)
	.fetch(&mut *connection)
	.map_ok(|region| region.into())
	.try_collect()
	.await
}

pub async fn mark_byoc_region_as_reconnected(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			disconnected_at = NULL,
			status = 'active'
		WHERE
			workspace_id IS NOT NULL AND
			id = $1;
		"#,
		region_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn mark_byoc_region_as_disconnected(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	disconnected_at: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			disconnected_at = $1,
			status = 'disconnected'
		WHERE
			workspace_id IS NOT NULL AND
			id = $2;
		"#,
		disconnected_at as _,
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
				status
			)
		VALUES
			($1, $2, $3, $4, $5);
		"#,
		region_id as _,
		name,
		cloud_provider as _,
		workspace_id as _,
		RegionStatus::Creating as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn mark_deployment_region_as_active(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	kube_config: Kubeconfig,
	ingress_hostname: &Host,
) -> Result<(), sqlx::Error> {
	let kube_config = sqlx::types::Json(kube_config);
	let ingress_hostname = ingress_hostname.to_string();

	query!(
		r#"
		UPDATE
			deployment_region
		SET
			status = 'active',
			config_file = $2,
			ingress_hostname = $3
		WHERE
			id = $1;
		"#,
		region_id as _,
		kube_config as _,
		ingress_hostname as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn mark_deployment_region_as_errored(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_region
		SET
			status = 'errored'
		WHERE
			id = $1;
		"#,
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
			config_file = NULL,
			ingress_hostname = NULL
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
