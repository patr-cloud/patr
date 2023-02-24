use api_models::{models::workspace::region::RegionStatus, utils::Uuid};
use eve_rs::AsError;
use kube::config::{
	AuthInfo,
	Cluster,
	Context,
	Kubeconfig,
	NamedAuthInfo,
	NamedCluster,
	NamedContext,
};
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
		ingress_ip_addr: String,
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
	pub kube_config: Kubeconfig,
}

pub fn get_kubernetes_config_for_default_region(
	config: &Settings,
) -> KubernetesConfigDetails {
	KubernetesConfigDetails {
		cluster_type: ClusterType::PatrOwned,
		kube_config: generate_kubeconfig_from_template(
			&config.kubernetes.cluster_url,
			&config.kubernetes.auth_token,
			&config.kubernetes.certificate_authority_data,
		),
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
		KubernetesConfigDetails {
			cluster_type: ClusterType::PatrOwned,
			kube_config: generate_kubeconfig_from_template(
				&config.kubernetes.cluster_url,
				&config.kubernetes.auth_token,
				&config.kubernetes.certificate_authority_data,
			),
		}
	} else {
		match (region.status, region.config_file, region.ingress_hostname) {
			(
				RegionStatus::Active,
				Some(config_file),
				Some(ingress_ip_addr),
			) => KubernetesConfigDetails {
				cluster_type: ClusterType::UserOwned {
					region_id: region.id,
					ingress_ip_addr,
				},
				kube_config: config_file,
			},
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
	min_nodes: u16,
	max_nodes: u16,
	auto_scale: bool,
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
			version: "latest".to_string(),
			name: cluster_name.to_string(),
			node_pools: vec![K8NodePool {
				name: node_name.to_string(),
				size: node_size_slug.to_string(),
				auto_scale,
				min_nodes,
				max_nodes,
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
	auth_token: &str,
	certificate_authority_data: &str,
) -> Kubeconfig {
	let cluster_name = "clusterId";
	let context_name = "contextId";
	let auth_info = "authInfoId";

	Kubeconfig {
		api_version: Some("v1".into()),
		kind: Some("Config".into()),
		clusters: vec![NamedCluster {
			name: cluster_name.into(),
			cluster: Cluster {
				server: cluster_url.into(),
				certificate_authority_data: Some(
					certificate_authority_data.into(),
				),
				certificate_authority: None,
				proxy_url: None,
				extensions: None,
				insecure_skip_tls_verify: None,
			},
		}],
		auth_infos: vec![NamedAuthInfo {
			name: auth_info.into(),
			auth_info: AuthInfo {
				token: Some(auth_token.to_owned().into()),
				..Default::default()
			},
		}],
		contexts: vec![NamedContext {
			name: context_name.into(),
			context: Context {
				cluster: cluster_name.into(),
				user: auth_info.into(),
				extensions: None,
				namespace: None,
			},
		}],
		current_context: Some(context_name.into()),
		..Default::default()
	}
}
