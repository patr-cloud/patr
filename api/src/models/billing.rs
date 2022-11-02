use api_models::{
	models::workspace::infrastructure::list_all_deployment_machine_type::DeploymentMachineType,
	utils::Uuid,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentBill {
	pub deployment_id: Uuid,
	pub deployment_name: String,
	pub bill_items: Vec<DeploymentBillItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentBillItem {
	pub machine_type: DeploymentMachineType, // CPU, RAM
	pub num_instances: u32,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseBill {
	pub database_id: Uuid,
	pub database_name: String,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticSiteBill {
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUrlBill {
	pub url_count: u64,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerRepositoryBill {
	pub storage: u64,
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainBill {
	pub hours: u64,
	pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretsBill {
	pub secrets_count: u64,
	pub hours: u64,
	pub amount: f64,
}
