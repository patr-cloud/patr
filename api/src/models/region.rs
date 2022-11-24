use api_models::utils::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct K8sConfig {
	pub region: String,
	pub api_token: String,
	pub cluster_name: String,
	pub num_node: u16,
	pub node_name: String,
	pub node_size: String,
	// TODO: add more info about high availablity and other stuff
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