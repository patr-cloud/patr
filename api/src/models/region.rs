use api_models::utils::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct K8sConfig {
	pub region: String,
	pub name: String,
	pub version: String,
	pub node_pools: Vec<K8NodePool>, /* TODO: add more info about high
	                                  * availablity and other stuff */
}

#[derive(Debug, Deserialize, Serialize)]
pub struct K8NodePool {
	pub count: u16,
	pub name: String,
	pub size: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct K8sCreateCluster {
	pub kubernetes_cluster: K8sClusterCreateInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct K8sClusterCreateInfo {
	pub id: Uuid,
	pub name: String,
	pub region: String,
	pub version: String,
	pub created_at: String,
	pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DOKubeConfig {}
