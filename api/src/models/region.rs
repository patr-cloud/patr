use api_models::utils::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct K8sConfig {
	pub region: String,
	pub name: String,
	pub version: String,
	pub node_pools: Vec<K8NodePool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct K8NodePool {
	pub name: String,
	pub size: String,
	pub auto_scale: bool,
	pub min_nodes: u16,
	pub max_nodes: u16,
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
