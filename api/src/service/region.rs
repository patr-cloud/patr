use api_models::{
	models::workspace::{
		domain::DnsRecordValue,
		region::{AddRegionToWorkspaceData, InfrastructureCloudProvider},
	},
	utils::Uuid,
};
use chrono::Utc;
use eve_rs::AsError;
use reqwest::Url;

use crate::{
	db,
	error,
	models::rbac::{self, resource_types},
	service,
	utils::{settings::Settings, Error},
	Database,
};

#[derive(Debug, Clone)]
pub struct KubernetesConfig {
	pub cluster_url: String,
	pub auth_username: String,
	pub auth_token: String,
	pub certificate_authority_data: String,
}

pub async fn get_kubernetes_config_for_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	config: &Settings,
) -> Result<KubernetesConfig, Error> {
	let region = db::get_region_by_id(connection, region_id)
		.await?
		.status(500)?;

	let kubeconfig = if region.workspace_id.is_none() {
		// use the patr clusters

		// for now returing the configured cluster credentials
		KubernetesConfig {
			cluster_url: config.kubernetes.cluster_url.to_owned(),
			auth_username: config.kubernetes.auth_username.to_owned(),
			auth_token: config.kubernetes.auth_token.to_owned(),
			certificate_authority_data: config
				.kubernetes
				.certificate_authority_data
				.to_owned(),
		}
	} else {
		match (
			region.ready,
			region.kubernetes_cluster_url,
			region.kubernetes_auth_username,
			region.kubernetes_auth_token,
			region.kubernetes_ca_data,
		) {
			(
				true,
				Some(cluster_url),
				Some(auth_username),
				Some(auth_token),
				Some(certificate_authority_data),
			) => KubernetesConfig {
				cluster_url,
				auth_username,
				auth_token,
				certificate_authority_data,
			},
			_ => {
				log::info!("cluster {region_id} is not yet initialized");
				return Err(Error::empty());
			}
		}
	};

	Ok(kubeconfig)
}

// TODO: remove it
pub async fn _add_new_region_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	name: &str,
	data: &AddRegionToWorkspaceData,
	config: &Settings,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	let region_id = db::generate_new_resource_id(connection).await?;

	db::create_resource(
		connection,
		&region_id,
		&format!("Region: {}", name),
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(resource_types::DEPLOYMENT_REGION)
			.unwrap(),
		workspace_id,
		&Utc::now(),
	)
	.await?;

	match data {
		AddRegionToWorkspaceData::Digitalocean {
			cloud_provider: _,
			region: _,
			api_token: _,
		} => todo!(),
		AddRegionToWorkspaceData::KubernetesCluster {
			certificate_authority_data,
			cluster_url,
			auth_username,
			auth_token,
		} => {
			db::add_deployment_region_to_workspace(
				connection,
				&region_id,
				name,
				&InfrastructureCloudProvider::Other,
				workspace_id,
			)
			.await?;

			let url = Url::parse(cluster_url)?;

			let patr_domain = db::get_domain_by_name(connection, "patr.cloud")
				.await?
				.status(500)?;

			let resource = db::get_resource_by_id(connection, &patr_domain.id)
				.await?
				.status(500)?;

			service::create_patr_domain_dns_record(
				connection,
				&resource.owner_id,
				&patr_domain.id,
				region_id.as_str(),
				0,
				&DnsRecordValue::CNAME {
					target: url
						.domain()
						.status(400)
						.body(error!(WRONG_PARAMETERS).to_string())?
						.to_string(),
					proxied: false,
				},
				config,
				request_id,
			)
			.await?;

			service::queue_setup_kubernetes_cluster(
				&region_id,
				cluster_url,
				certificate_authority_data,
				auth_username,
				auth_token,
				config,
				request_id,
			)
			.await?;
		}
	}

	Ok(region_id)
}
