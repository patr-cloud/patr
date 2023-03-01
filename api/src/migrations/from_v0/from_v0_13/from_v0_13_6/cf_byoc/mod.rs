mod cf_models;
mod cf_utils;
mod k8s_migrations;

use std::{collections::HashMap, time::Duration};

use api_models::utils::Uuid;
use cloudflare::endpoints::workerskv::write_bulk::KeyValuePair;
use futures::TryStreamExt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
	migrate_query as query,
	migrations::from_v0::from_v0_13::from_v0_13_6::cf_byoc::{
		cf_models::routing::{RouteType, UrlType},
		k8s_migrations::{
			delete_k8s_certificate_resources,
			delete_k8s_managed_url_resources,
			delete_k8s_region_resources,
			delete_k8s_static_site_resources,
			patch_ingress_for_default_region_deployments,
		},
	},
	utils::{settings::Settings, Error},
	Database,
};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	log::info!("Running cloudflare ingress migrations");

	// manual migrations before deploying:
	//  - sync s3 buckets from do to r2
	//  - delete byoc dns records from {region-id}.patr.cloud
	//  - create origin ca cert for sgp cluster and set it as default

	migrate_byoc_region(connection, config).await?;
	migrate_workspace_domain(connection, config).await?;
	migrate_managed_url(connection, config).await?;

	update_cloudflare_kv_for_deployments(connection, config).await?;
	update_cloudflare_kv_for_static_sites(connection, config).await?;
	update_cloudflare_kv_for_managed_urls(connection, config).await?;

	delete_k8s_region_resources(connection, config).await?;
	delete_k8s_static_site_resources(connection, config).await?;
	delete_k8s_managed_url_resources(connection, config).await?;
	delete_k8s_certificate_resources(connection, config).await?;
	patch_ingress_for_default_region_deployments(connection, config).await?;

	log::info!("Completed cloudflare ingress migrations");
	Ok(())
}

async fn migrate_byoc_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	log::info!("Running byoc region migrations for cf ingress");

	query!(
		r#"
		ALTER TABLE deployment_region
		RENAME TO region;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// 1. Delete resources associated with byoc region

	// no managed url is pointed to byoc region deployments,
	// so no need to handle it in migrations

	// delete the byoc deployments
	let byoc_deployments = query!(
		r#"
		SELECT id
		FROM deployment
		WHERE region IN (
			SELECT id
			FROM region
			WHERE workspace_id IS NOT NULL
		);
		"#
	)
	.fetch(&mut *connection)
	.map_ok(|row| row.get::<Uuid, _>("id"))
	.try_collect::<Vec<_>>()
	.await?;

	for (idx, deployment_id) in byoc_deployments.iter().enumerate() {
		log::info!(
			"{}/{} - Marking BYOC deployment `{}` as deleted",
			idx,
			byoc_deployments.len(),
			deployment_id,
		);

		query!(
			r#"
			UPDATE
				deployment
			SET
				deleted = NOW(),
				status = 'deleted'
			WHERE
				id = $1;
			"#,
			deployment_id
		)
		.execute(&mut *connection)
		.await?;

		// audit log for deployment is not added,
		// as it is not maintained properly
	}

	// 2. Delete byoc region

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
		ALTER TABLE region
			DROP CONSTRAINT deployment_region_chk_ready_or_not,
			ADD COLUMN status REGION_STATUS,
			ADD COLUMN ingress_hostname TEXT,
			ADD COLUMN cloudflare_certificate_id TEXT,
			ADD COLUMN config_file JSON,
			ADD COLUMN deleted TIMESTAMPTZ,
			ADD COLUMN disconnected_at TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	log::info!("Marking all BYOC region as deleted");
	query!(
		r#"
		UPDATE region
		SET status =
			CASE
				WHEN (workspace_id IS NULL AND ready = true)
					THEN 'active'::REGION_STATUS
				WHEN (workspace_id IS NULL AND ready = false)
					THEN 'coming_soon'::REGION_STATUS
				ELSE 'deleted'::REGION_STATUS
			END;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE region
		SET deleted = NOW()
		WHERE status = 'deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE region
			DROP COLUMN ready,
			DROP COLUMN kubernetes_cluster_url,
			DROP COLUMN kubernetes_auth_username,
			DROP COLUMN kubernetes_auth_token,
			DROP COLUMN kubernetes_ca_data,
			DROP COLUMN kubernetes_ingress_ip_addr,
			ALTER COLUMN status SET NOT NULL,
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
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX region_uq_workspace_id_name
		ON region(workspace_id, name)
		WHERE
			deleted IS NULL AND
			workspace_id IS NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE region
			ADD CONSTRAINT region_fk_id_workspace_id
			FOREIGN KEY (id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	log::info!("Completed byoc region migrations for cf ingress");
	Ok(())
}

async fn migrate_workspace_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	log::info!("Running workspace domain migrations for cf ingress");

	query!(
		r#"
		ALTER TABLE workspace_domain
			ADD COLUMN cloudflare_worker_route_id TEXT NOT NULL
				DEFAULT 'already_deleted';
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
			ALTER COLUMN cloudflare_worker_route_id DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	let valid_workspace_domain = query!(
		r#"
		SELECT
			workspace_domain.id as "domain_id",
			concat(domain.name, '.', domain.tld) AS "domain_name"
		FROM workspace_domain
		JOIN domain
			ON workspace_domain.id = domain.id
		WHERE domain.deleted IS NULL;
		"#
	)
	.fetch(&mut *connection)
	.map_ok(|row| {
		(
			row.get::<Uuid, _>("domain_id"),
			row.get::<String, _>("domain_name"),
		)
	})
	.try_collect::<Vec<_>>()
	.await?;

	for (idx, (domain_id, domain_name)) in
		valid_workspace_domain.iter().enumerate()
	{
		log::info!(
			"{}/{} - Creating cf worker routes for domain {} with name `{}`",
			idx,
			valid_workspace_domain.len(),
			domain_name,
			domain_id,
		);

		tokio::time::sleep(Duration::from_millis(300)).await;
		let cloudflare_worker_route_id =
			cf_utils::create_cf_worker_routes_for_domain(domain_name, config)
				.await?;

		query!(
			r#"
			UPDATE
				workspace_domain
			SET
				cloudflare_worker_route_id = $2
			WHERE
				id = $1;
			"#,
			&domain_id,
			&cloudflare_worker_route_id
		)
		.execute(&mut *connection)
		.await?;
	}

	log::info!("Completed workspace domain migrations for cf ingress");
	Ok(())
}

async fn migrate_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	log::info!("Running managed url migrations for cf ingress");

	query!(
		r#"
		ALTER TABLE managed_url
			ADD COLUMN cf_custom_hostname_id TEXT NOT NULL
				DEFAULT 'already_deleted';;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
			ADD COLUMN cf_custom_hostname_id DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	let valid_custom_hostnames = query!(
			r#"
			SELECT
				DISTINCT CONCAT(managed_url.sub_domain, '.', domain.name, '.', domain.tld) as "hostname"
			FROM managed_url
			JOIN workspace_domain
				ON workspace_domain.id = managed_url.domain_id
			JOIN domain
				ON domain.id = workspace_domain.id
			WHERE
				managed_url.deleted IS NULL;
			"#
		)
		.fetch(&mut *connection)
		.map_ok(|row| {
				row.get::<String, _>("hostname")
		})
		.try_collect::<Vec<_>>()
		.await?;

	for (idx, hostname) in valid_custom_hostnames.iter().enumerate() {
		log::info!(
			" {}/{} - Creating cf custom hostname for {}",
			idx,
			valid_custom_hostnames.len(),
			hostname
		);

		let cf_hostname = hostname.strip_prefix("@.").unwrap_or(hostname);
		tokio::time::sleep(Duration::from_millis(300)).await;
		let cf_custom_hostname_id =
			cf_utils::create_cf_custom_hostname(cf_hostname, config).await?;

		query!(
			r#"
				UPDATE managed_url
				SET cf_custom_hostname_id = $2
				FROM domain
				WHERE
					managed_url.deleted IS NULL AND
					domain.id = managed_url.domain_id AND
					CONCAT(managed_url.sub_domain, '.', domain.name, '.', domain.tld) = $1;
				"#,
			hostname,
			&cf_custom_hostname_id
		)
		.execute(&mut *connection)
		.await?;
	}

	log::info!("Completed managed url migrations for cf ingress");
	Ok(())
}

#[derive(
	Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, sqlx::Type,
)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "DEPLOYMENT_STATUS", rename_all = "lowercase")]
pub enum DeploymentStatus {
	Created,
	Pushed,
	Deploying,
	Running,
	Stopped,
	Errored,
	Deleted,
}

async fn update_cloudflare_kv_for_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	log::info!("Updating deployment kv for cf ingress");

	let default_region_id = query!(
		r#"
		SELECT
			id as "id: Uuid"
		FROM
			region
		WHERE
			name = 'Singapore'
			AND provider = 'digitalocean'
			AND workspace_id IS NULL
			AND status = 'active';
		"#
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.get::<Uuid, _>("id"))
	.expect("Default region should be present already");

	// byoc deployments will be in deleted state first,
	// so need to consider only patr deployments
	let deployment_details = query!(
		r#"
		SELECT
			deployment.id,
			deployment.status,
			deployment_exposed_port.port
		FROM deployment
		LEFT JOIN deployment_exposed_port
			ON deployment_exposed_port.deployment_id = deployment.id
		WHERE
			deployment.status != 'deleted' AND
			deployemnt.deleted IS NULL;
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<DeploymentStatus, _>("status"),
			row.get::<Option<i32>, _>("port"),
		)
	})
	.try_collect::<Vec<_>>()
	.await?;

	let deployment_details = deployment_details.into_iter().fold(
		HashMap::<(Uuid, DeploymentStatus), Vec<u16>>::new(),
		|mut accu, (id, status, port)| {
			if let Some(port) = port {
				accu.entry((id, status)).or_default().push(port as u16);
			}
			accu
		},
	);

	let total_count = deployment_details.len();
	let mut next_idx = 1;

	for chunk in &deployment_details.into_iter().chunks(500) {
		let kv = chunk
			.map(|((deployment_id, deployment_status), deployed_ports)| {
				let key = cf_models::deployment::Key(deployment_id).to_string();
				let value = serde_json::to_string(&match deployment_status {
					DeploymentStatus::Created => {
						cf_models::deployment::Value::Created
					}
					DeploymentStatus::Deploying |
					DeploymentStatus::Pushed |
					DeploymentStatus::Running |
					DeploymentStatus::Errored => cf_models::deployment::Value::Running {
						region_id: default_region_id.clone(),
						ports: deployed_ports,
					},
					DeploymentStatus::Stopped => {
						cf_models::deployment::Value::Stopped
					}
					DeploymentStatus::Deleted => {
						cf_models::deployment::Value::Deleted
					}
				})
				.expect("Serialization should not fail");

				KeyValuePair {
					key,
					value,
					expiration: None,
					expiration_ttl: None,
					base64: None,
				}
			})
			.collect::<Vec<_>>();

		log::info!(
			"Updating KV for deployments: {}-{}/{}",
			next_idx,
			next_idx + kv.len(),
			total_count,
		);
		next_idx += kv.len();
		tokio::time::sleep(Duration::from_millis(300)).await;
		cf_utils::update_kv_for_deployment(kv, config).await?;
	}

	log::info!("Updated deployment kv for cf ingress");

	Ok(())
}

async fn update_cloudflare_kv_for_static_sites(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	log::info!("Updating static site kv for cf ingress");

	let static_site_details = query!(
		r#"
		SELECT
			static_site.id,
			static_site.status,
			static_site.current_live_upload
		FROM static_site
		WHERE 
			status != 'deleted' AND
			deleted IS NULL;
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<DeploymentStatus, _>("status"),
			row.get::<Option<Uuid>, _>("current_live_upload"),
		)
	})
	.try_collect::<Vec<_>>()
	.await?;

	let total_count = static_site_details.len();
	let mut next_idx = 1;

	for chunk in &static_site_details.into_iter().chunks(500) {
		let kv = chunk
			.map(
				|(static_site_id, static_site_status, current_live_upload)| {
					let key =
						cf_models::static_site::Key(static_site_id).to_string();
					let value = serde_json::to_string(&match (
						static_site_status,
						current_live_upload,
					) {
						(DeploymentStatus::Created, _) => {
							cf_models::static_site::Value::Created
						}
						(DeploymentStatus::Running, Some(upload_id)) => {
							cf_models::static_site::Value::Serving(upload_id)
						}
						(DeploymentStatus::Deleted, _) => {
							cf_models::static_site::Value::Deleted
						}
						_ => cf_models::static_site::Value::Stopped,
					})
					.expect("Serialization should not fail");

					KeyValuePair {
						key,
						value,
						expiration: None,
						expiration_ttl: None,
						base64: None,
					}
				},
			)
			.collect::<Vec<_>>();

		log::info!(
			"Updating KV for static sites: {}-{}/{}",
			next_idx,
			next_idx + kv.len(),
			total_count,
		);
		next_idx += kv.len();
		tokio::time::sleep(Duration::from_millis(300)).await;
		cf_utils::update_kv_for_static_site(kv, config).await?;
	}

	log::info!("Updated static site kv for cf ingress");

	Ok(())
}

async fn update_cloudflare_kv_for_managed_urls(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	log::info!("Updating managed url kv for cf ingress");

	#[derive(sqlx::Type)]
	#[sqlx(type_name = "MANAGED_URL_TYPE", rename_all = "snake_case")]
	pub enum ManagedUrlType {
		ProxyToDeployment,
		ProxyToStaticSite,
		ProxyUrl,
		Redirect,
	}

	let managed_url_details = query!(
		r#"
		SELECT
			managed_url.sub_domain,
			CONCAT(domain.name, '.', domain.tld) AS "domain",
			managed_url.path,
			managed_url.url_type,
			managed_url.deployment_id,
			managed_url.port,
			managed_url.static_site_id,
			managed_url.url,
			managed_url.permanent_redirect,
			managed_url.http_only
		FROM
			managed_url
		JOIN workspace_domain
			ON workspace_domain.id = managed_url.domain_id
		JOIN domain
			ON domain.id = workspace_domain.id
		WHERE
			managed_url.deleted IS NULL AND
			domain.deleted IS NULL
		ORDER BY
			managed_url.domain_id,
			managed_url.sub_domain,
			path DESC;
		"#
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<String, _>("sub_domain"),
			row.get::<String, _>("domain"),
			row.get::<String, _>("path"),
			row.get::<ManagedUrlType, _>("url_type"),
			row.get::<Option<Uuid>, _>("deployment_id"),
			row.get::<Option<i32>, _>("port"),
			row.get::<Option<Uuid>, _>("static_site_id"),
			row.get::<Option<String>, _>("url"),
			row.get::<Option<bool>, _>("permanent_redirect"),
			row.get::<Option<bool>, _>("http_only"),
		)
	})
	.filter_map(
		|(
			sub_domain,
			domain,
			path,
			url_type,
			deployment_id,
			port,
			static_site_id,
			url,
			permanent_redirect,
			http_only,
		)| {
			let url_type = match url_type {
				ManagedUrlType::ProxyToDeployment => UrlType::ProxyDeployment {
					deployment_id: deployment_id?,
					port: port.and_then(|port| TryFrom::try_from(port).ok())?,
				},
				ManagedUrlType::ProxyToStaticSite => UrlType::ProxyStaticSite {
					static_site_id: static_site_id?,
				},
				ManagedUrlType::ProxyUrl => UrlType::ProxyUrl {
					url: url?,
					http_only: http_only?,
				},
				ManagedUrlType::Redirect => UrlType::Redirect {
					url: url?,
					permanent_redirect: permanent_redirect?,
					http_only: http_only?,
				},
			};

			Some(((sub_domain, domain), RouteType { path, url_type }))
		},
	)
	.fold(
		HashMap::<(String, String), Vec<RouteType>>::new(),
		|mut accu, (host, route_type)| {
			accu.entry(host).or_default().push(route_type);
			accu
		},
	);

	let total_count = managed_url_details.len();
	let mut next_idx = 1;

	for chunk in &managed_url_details.into_iter().chunks(50) {
		let kv = chunk
			.map(|((sub_domain, domain), route_types)| {
				let key =
					cf_models::routing::Key { sub_domain, domain }.to_string();
				let value = serde_json::to_string(&cf_models::routing::Value(
					route_types,
				))
				.expect("Serialization should not fail");

				KeyValuePair {
					key,
					value,
					expiration: None,
					expiration_ttl: None,
					base64: None,
				}
			})
			.collect::<Vec<_>>();

		log::info!(
			"Updating KV for managed url host: {}-{}/{}",
			next_idx,
			next_idx + kv.len(),
			total_count,
		);
		next_idx += kv.len();
		tokio::time::sleep(Duration::from_millis(300)).await;
		cf_utils::update_kv_for_managed_url(kv, config).await?;
	}

	log::info!("Updated managed url kv for cf ingress");

	Ok(())
}
