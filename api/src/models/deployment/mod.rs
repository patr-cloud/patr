use std::collections::HashMap;

use api_models::{
	models::workspace::infrastructure::DeploymentCloudProvider,
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

lazy_static::lazy_static! {
	pub static ref DEFAULT_DEPLOYMENT_REGIONS: Vec<DefaultDeploymentRegion> = vec![
		DefaultDeploymentRegion {
			name: "Asia",
			cloud_provider: None,
			coordinates: None,
			slug: None,
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "Singapore",
					cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
					coordinates: Some((1.3521, 103.8198)),
					slug: None,
					child_regions: vec![],
				},
				DefaultDeploymentRegion {
					name: "India",
					cloud_provider: None,
					coordinates: None,
					slug: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Bangalore",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((2.9716, 77.5946)),
						slug: Some("do-blr1"),
						child_regions: vec![],
					}],
				},
			],
		},
		DefaultDeploymentRegion {
			name: "Europe",
			cloud_provider: None,
			coordinates: None,
			slug: None,
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "England",
					cloud_provider: None,
					coordinates: None,
					slug: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "London",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((51.5072, 0.1276)),
						slug: None,
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "Netherlands",
					cloud_provider: None,
					coordinates: None,
					slug: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Amsterdam",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((52.3676, 4.9041)),
						slug: None,
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "Germany",
					cloud_provider: None,
					coordinates: None,
					slug: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Frankfurt",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((50.1109, 8.6821)),
						slug: None,
						child_regions: vec![],
					}],
				},
			],
		},
		DefaultDeploymentRegion {
			name: "North-America",
			cloud_provider: None,
			coordinates: None,
			slug: None,
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "Canada",
					cloud_provider: None,
					coordinates: None,
					slug: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Toronto",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((43.6532, 79.3832)),
						slug: None,
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "USA",
					cloud_provider: None,
					coordinates: None,
					slug: None,
					child_regions: vec![
						DefaultDeploymentRegion {
							name: "New-York 1",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((40.7128, 74.0060)),
							slug: None,
							child_regions: vec![],
						},
						DefaultDeploymentRegion {
							name: "New-York 2",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((40.7128, 74.0060)),
							slug: None,
							child_regions: vec![],
						},
						DefaultDeploymentRegion {
							name: "San Francisco",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((37.7749, 122.4194)),
							slug: None,
							child_regions: vec![],
						},
					],
				},
			],
		},
	];
}

pub static MACHINE_TYPES: OnceCell<HashMap<Uuid, (i16, i32)>> = OnceCell::new();
pub static REGIONS: OnceCell<
	HashMap<Uuid, (String, Option<DeploymentCloudProvider>)>,
> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct DefaultDeploymentRegion {
	pub name: &'static str,
	pub cloud_provider: Option<DeploymentCloudProvider>,
	pub coordinates: Option<(f64, f64)>,
	pub slug: Option<&'static str>,
	pub child_regions: Vec<DefaultDeploymentRegion>,
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
