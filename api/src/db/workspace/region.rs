use api_models::{
	models::workspace::region::{InfrastructureCloudProvider, RegionStatus},
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use kube::config::{
	AuthInfo,
	Cluster,
	Context,
	Kubeconfig,
	NamedAuthInfo,
	NamedCluster,
	NamedContext,
};
use sqlx::types::Json;
use url::Host;

use crate::{
	models::deployment::DEFAULT_DEPLOYMENT_REGIONS,
	query,
	query_as,
	Database,
};

#[derive(Debug)]
pub struct Region {
	pub id: Uuid,
	pub name: String,
	pub cloud_provider: InfrastructureCloudProvider,
	pub workspace_id: Option<Uuid>,
	pub ingress_hostname: Option<String>,
	pub message_log: Option<String>,
	pub cloudflare_certificate_id: Option<String>,
	pub config_file: Option<Json<Kubeconfig>>,
	pub status: RegionStatus,
	pub disconnected_at: Option<DateTime<Utc>>,
}

impl Region {
	pub fn is_byoc_region(&self) -> bool {
		self.workspace_id.is_some()
	}

	pub fn is_patr_region(&self) -> bool {
		!self.is_byoc_region()
	}

	pub fn is_ready(&self) -> bool {
		self.status == RegionStatus::Active || self.is_patr_region()
	}
}

pub async fn initialize_region_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing region tables");
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
			id UUID CONSTRAINT region_pk PRIMARY KEY,
			name TEXT NOT NULL,
			provider INFRASTRUCTURE_CLOUD_PROVIDER NOT NULL,
			workspace_id UUID CONSTRAINT region_fk_workspace_id
				REFERENCES workspace(id),
			message_log TEXT,
			status REGION_STATUS NOT NULL,
			ingress_hostname TEXT,
			cloudflare_certificate_id TEXT,
			config_file JSON,
			deleted TIMESTAMPTZ,
			disconnected_at TIMESTAMPTZ,
			CONSTRAINT region_chk_status CHECK(
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
			)
		);
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

pub async fn initialize_region_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE region
		ADD CONSTRAINT region_fk_id_workspace_id
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
					region
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

		if region.status == RegionStatus::Active {
			let config = crate::service::get_settings();
			let kubeconfig = Kubeconfig {
				api_version: Some("v1".to_string()),
				kind: Some("Config".to_string()),
				clusters: vec![NamedCluster {
					name: "kubernetesCluster".to_owned(),
					cluster: Some(Cluster {
						server: Some(config.kubernetes.cluster_url.to_owned()),
						certificate_authority_data: Some(
							config
								.kubernetes
								.certificate_authority_data
								.to_owned(),
						),
						insecure_skip_tls_verify: None,
						certificate_authority: None,
						proxy_url: None,
						extensions: None,
						..Default::default()
					}),
				}],
				auth_infos: vec![NamedAuthInfo {
					name: config.kubernetes.auth_username.to_owned(),
					auth_info: Some(AuthInfo {
						token: Some(
							config.kubernetes.auth_token.to_owned().into(),
						),
						..Default::default()
					}),
				}],
				contexts: vec![NamedContext {
					name: "kubernetesContext".to_owned(),
					context: Some(Context {
						cluster: "kubernetesCluster".to_owned(),
						user: config.kubernetes.auth_username.to_owned(),
						extensions: None,
						namespace: None,
					}),
				}],
				current_context: Some("kubernetesContext".to_owned()),
				preferences: None,
				extensions: None,
			};

			query!(
				r#"
				INSERT INTO
					region(
						id,
						name,
						provider,
						status,
						config_file,
						ingress_hostname,
						cloudflare_certificate_id
					)
				VALUES
					($1, $2, $3, $4, $5, '', '');
				"#,
				region_id as _,
				region.name,
				region.cloud_provider as _,
				region.status as _,
				Json(kubeconfig) as _
			)
			.execute(&mut *connection)
			.await?;
		} else {
			query!(
				r#"
				INSERT INTO
					region(
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
	}

	Ok(())
}

pub async fn get_region_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<Option<Region>, sqlx::Error> {
	query_as!(
		Region,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			cloudflare_certificate_id,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			region
		WHERE
			id = $1;
		"#,
		region_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_all_default_regions(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<Region>, sqlx::Error> {
	query_as!(
		Region,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			cloudflare_certificate_id,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			region
		WHERE
			workspace_id IS NULL;
		"#,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_region_by_name_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	name: &str,
	workspace_id: &Uuid,
) -> Result<Option<Region>, sqlx::Error> {
	query_as!(
		Region,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			cloudflare_certificate_id,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			region
		WHERE
			name = $1 AND
			workspace_id = $2 AND
			status != 'deleted';
		"#,
		name as _,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_all_regions_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Region>, sqlx::Error> {
	query_as!(
		Region,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			cloudflare_certificate_id,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			region
		WHERE
			status != 'deleted' AND
			(
				workspace_id IS NULL OR
				workspace_id = $1
			);
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_byoc_regions_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Region>, sqlx::Error> {
	query_as!(
		Region,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			cloudflare_certificate_id,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			region
		WHERE
			status != 'deleted' AND
			workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_active_byoc_region(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<Region>, sqlx::Error> {
	query_as!(
		Region,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			cloudflare_certificate_id,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			region
		WHERE
			workspace_id IS NOT NULL AND
			status = 'active';
		"#,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_disconnected_byoc_region(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<Region>, sqlx::Error> {
	query_as!(
		Region,
		r#"
		SELECT
			id as "id: _",
			name,
			provider as "cloud_provider: _",
			workspace_id as "workspace_id: _",
			ingress_hostname as "ingress_hostname: _",
			message_log,
			cloudflare_certificate_id,
			config_file as "config_file: _",
			status as "status: _",
			disconnected_at as "disconnected_at: _"
		FROM
			region
		WHERE
			workspace_id IS NOT NULL AND
			status = 'disconnected' AND
			disconnected_at IS NOT NULL
		ORDER BY disconnected_at;
		"#,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn set_region_as_connected(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			region
		SET
			disconnected_at = NULL,
			status = 'active'
		WHERE
			id = $1;
		"#,
		region_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_region_as_disconnected(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	disconnected_at: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			region
		SET
			disconnected_at = $1,
			status = 'disconnected'
		WHERE
			id = $2;
		"#,
		disconnected_at as _,
		region_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_region_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	name: &str,
	cloud_provider: &InfrastructureCloudProvider,
	workspace_id: &Uuid,
	cloudflare_certificate_id: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			region(
				id,
				name,
				provider,
				workspace_id,
				cloudflare_certificate_id,
				status
			)
		VALUES
			($1, $2, $3, $4, $5, 'creating');
		"#,
		region_id as _,
		name,
		cloud_provider as _,
		workspace_id as _,
		cloudflare_certificate_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_region_as_active(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	kube_config: Kubeconfig,
	ingress_hostname: &Host,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			region
		SET
			status = 'active',
			config_file = $2,
			ingress_hostname = $3,
			disconnected_at = NULL
		WHERE
			id = $1;
		"#,
		region_id as _,
		Json(kube_config) as _,
		ingress_hostname.to_string(),
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_region_as_errored(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			region
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
			region
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
			region
		SET
			deleted = $2,
			status = 'deleted'
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
