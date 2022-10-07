use std::collections::HashMap;

use api_models::utils::Uuid;
use serde::{Deserialize, Serialize};

use crate::db::{DomainPlan, StaticSitePlan};

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
	pub machine_type: (u16, u32), // CPU, RAM
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUsageBill {
	pub total_cost: f64,
	pub deployment_usages: HashMap<Uuid, DeploymentBill>,
	pub database_usages: HashMap<Uuid, DatabaseBill>,
	pub static_sites_usages: HashMap<StaticSitePlan, StaticSiteBill>,
	pub managed_url_usages: HashMap<u64, ManagedUrlBill>,
	pub docker_repository_usages: Vec<DockerRepositoryBill>,
	pub domains_usages: HashMap<DomainPlan, DomainBill>,
	pub secrets_usages: HashMap<u64, SecretsBill>,
}
