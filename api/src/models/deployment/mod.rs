use std::collections::HashMap;

use api_models::{
	models::workspace::region::{InfrastructureCloudProvider, RegionStatus},
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

pub mod cloud_providers;

pub const DEFAULT_MACHINE_TYPES: [(i16, i32); 5] = [
	(1, 2),  // 1 vCPU, 0.5 GB RAM
	(1, 4),  // 1 vCPU, 1 GB RAM
	(1, 8),  // 1 vCPU, 2 GB RAM
	(2, 8),  // 2 vCPU, 4 GB RAM
	(4, 32), // 4 vCPU, 8 GB RAM
];

pub const DEFAULT_DEPLOYMENT_REGIONS: [DefaultDeploymentRegion; 9] = [
	DefaultDeploymentRegion {
		name: "Singapore",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::Active,
	},
	DefaultDeploymentRegion {
		name: "Bangalore",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::ComingSoon,
	},
	DefaultDeploymentRegion {
		name: "London",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::ComingSoon,
	},
	DefaultDeploymentRegion {
		name: "Amsterdam",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::ComingSoon,
	},
	DefaultDeploymentRegion {
		name: "Frankfurt",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::ComingSoon,
	},
	DefaultDeploymentRegion {
		name: "Toronto",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::ComingSoon,
	},
	DefaultDeploymentRegion {
		name: "New-York 1",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::ComingSoon,
	},
	DefaultDeploymentRegion {
		name: "New-York 2",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::ComingSoon,
	},
	DefaultDeploymentRegion {
		name: "San Francisco",
		cloud_provider: InfrastructureCloudProvider::Digitalocean,
		status: RegionStatus::ComingSoon,
	},
];

pub static MACHINE_TYPES: OnceCell<HashMap<Uuid, (i16, i32)>> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct DefaultDeploymentRegion {
	pub name: &'static str,
	pub cloud_provider: InfrastructureCloudProvider,
	pub status: RegionStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusResponse {
	pub status: String,
	pub data: PrometheusData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusData {
	pub result_type: String,
	pub result: Vec<PrometheusMetrics>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrometheusMetrics {
	pub metric: PodName,
	pub values: Vec<Metric>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PodName {
	pub pod: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metric {
	pub timestamp: u64,
	pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logs {
	pub data: LokiData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LokiData {
	pub result: Vec<LokiResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LokiResult {
	pub values: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct DeploymentAuditLog {
	pub user_id: Option<Uuid>,
	pub ip_address: String,
	pub login_id: Option<Uuid>,
	pub workspace_audit_log_id: Uuid,
	pub patr_action: bool,
	pub time_now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeploymentBuildLog {
	pub pod: String,
	pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerInfo {
	pub customer: BillingInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingInfo {
	pub billing_address: Option<BillingInfoData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingInfoData {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub first_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub last_name: Option<String>,
	pub line1: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub line2: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub line3: Option<String>,
	pub city: String,
	pub state: String,
	pub zip: String,
	pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesEventData {
	pub reason: String,
	pub message: String,
	pub involved_object: InvolvedObject,
	pub r#type: String,
	pub first_timestamp: DateTime<Utc>,
	pub last_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvolvedObject {
	pub kind: String,
	pub namespace: String,
	pub name: String,
	pub labels: Option<Labels>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Labels {
	pub deployment_id: Option<String>,
	pub workspace_id: Option<String>,
}
