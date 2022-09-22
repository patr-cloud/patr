use std::net::IpAddr;

use api_models::utils::Uuid;
use eve_rs::AsError;

use crate::{
	db,
	utils::{settings::Settings, Error},
	Database,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterType {
	PatrOwned,
	UserOwned {
		region_id: Uuid,
		ingress_ip_addr: IpAddr,
	},
}

#[derive(Debug, Clone)]
pub struct KubernetesAuthDetails {
	pub cluster_url: String,
	pub auth_username: String,
	pub auth_token: String,
	pub certificate_authority_data: String,
}

#[derive(Debug, Clone)]
pub struct KubernetesConfigDetails {
	pub cluster_type: ClusterType,
	pub auth_details: KubernetesAuthDetails,
}

pub fn get_kubernetes_config_for_default_region(
	config: &Settings,
) -> KubernetesConfigDetails {
	KubernetesConfigDetails {
		cluster_type: ClusterType::PatrOwned,
		auth_details: KubernetesAuthDetails {
			cluster_url: config.kubernetes.cluster_url.to_owned(),
			auth_username: config.kubernetes.auth_username.to_owned(),
			auth_token: config.kubernetes.auth_token.to_owned(),
			certificate_authority_data: config
				.kubernetes
				.certificate_authority_data
				.to_owned(),
		},
	}
}

pub async fn get_kubernetes_config_for_region(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
	config: &Settings,
) -> Result<KubernetesConfigDetails, Error> {
	let region = db::get_region_by_id(connection, region_id)
		.await?
		.status(500)?;

	let kubeconfig = if region.workspace_id.is_none() {
		// use the patr clusters

		// for now returing the default cluster credentials
		get_kubernetes_config_for_default_region(config)
	} else {
		match (
			region.ready,
			region.kubernetes_cluster_url,
			region.kubernetes_auth_username,
			region.kubernetes_auth_token,
			region.kubernetes_ca_data,
			region.kubernetes_ingress_ip_addr,
		) {
			(
				true,
				Some(cluster_url),
				Some(auth_username),
				Some(auth_token),
				Some(certificate_authority_data),
				Some(ingress_ip_addr),
			) => KubernetesConfigDetails {
				cluster_type: ClusterType::UserOwned {
					region_id: region.id,
					ingress_ip_addr,
				},
				auth_details: KubernetesAuthDetails {
					cluster_url,
					auth_username,
					auth_token,
					certificate_authority_data,
				},
			},
			_ => {
				log::info!("cluster {region_id} is not yet initialized");
				return Err(Error::empty());
			}
		}
	};

	Ok(kubeconfig)
}

pub async fn is_deployed_on_patr_cluster(
	connection: &mut <Database as sqlx::Database>::Connection,
	region_id: &Uuid,
) -> Result<bool, Error> {
	let region = db::get_region_by_id(connection, region_id)
		.await?
		.status(500)?;
	Ok(region.workspace_id.is_none())
}
