use std::net::IpAddr;

use api_models::utils::Uuid;
use eve_rs::AsError;
use reqwest::Client;

use crate::{
	db,
	models::{K8NodePool, K8sConfig, K8sCreateCluster},
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
) -> Result<(ClusterType, String), Error> {
	let region = db::get_region_by_id(connection, region_id)
		.await?
		.status(500)?;

	let kubeconfig = if region.workspace_id.is_none() {
		// use the patr clusters

		// for now returing the default cluster credentials
		(
			ClusterType::PatrOwned,
			generate_kubeconfig_from_template(
				&config.kubernetes.cluster_url,
				&config.kubernetes.auth_username,
				&config.kubernetes.auth_token,
				&config.kubernetes.certificate_authority_data,
			),
		)
	} else {
		match (
			region.ready,
			region.config_file,
			region.kubernetes_ingress_ip_addr,
		) {
			(true, Some(config_file), Some(ingress_ip_addr)) => (
				ClusterType::UserOwned {
					region_id: region.id,
					ingress_ip_addr,
				},
				config_file,
			),
			_ => {
				log::info!("cluster {region_id} is not yet initialized");
				return Err(Error::empty().body(format!(
					"cluster {region_id} is not yet initialized"
				)));
			}
		}
	};

	Ok(kubeconfig)
}

pub async fn create_do_k8s_cluster(
	region: &str,
	api_token: &str,
	cluster_name: &str,
	num_node: &u16,
	node_name: &str,
	node_size_slug: &str,
	request_id: &Uuid,
) -> Result<Uuid, Error> {
	log::trace!(
		"request_id: {} creating digital ocean k8s cluster using api call",
		request_id
	);
	let client = Client::new();

	let k8s_cluster_id = client
		.post("https://api.digitalocean.com/v2/kubernetes/clusters")
		.bearer_auth(api_token)
		.json(&K8sConfig {
			region: region.to_string(),
			version: "1.24".to_string(),
			name: cluster_name.to_string(),
			node_pools: vec![K8NodePool {
				count: num_node.to_owned(),
				name: node_name.to_string(),
				size: node_size_slug.to_string(),
			}],
		})
		.send()
		.await?
		.json::<K8sCreateCluster>()
		.await?
		.kubernetes_cluster
		.id;

	Ok(k8s_cluster_id)
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

pub fn generate_kubeconfig_from_template(
	cluster_url: &str,
	auth_username: &str,
	auth_token: &str,
	certificate_authority_data: &str,
) -> String {
	format!(
		r#"apiVersion: v1
kind: Config
clusters:
  - name: kubernetesCluster
    cluster:
      certificate-authority-data: {certificate_authority_data}
      server: {cluster_url}
users:
  - name: {auth_username}
    user:
      token: {auth_token}
contexts:
  - name: kubernetesContext
    context:
      cluster: kubernetesCluster
      user: {auth_username}
current-context: kubernetesContext
preferences: {{}}"#
	)
}
