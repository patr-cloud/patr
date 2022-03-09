use std::collections::HashMap;

use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use super::db_mapping::DeploymentCloudProvider;

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
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "Singapore",
					cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
					coordinates: Some((1.3521, 103.8198)),
					child_regions: vec![],
				},
				DefaultDeploymentRegion {
					name: "India",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Bangalore",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((2.9716, 77.5946)),
						child_regions: vec![],
					}],
				},
			],
		},
		DefaultDeploymentRegion {
			name: "Europe",
			cloud_provider: None,
			coordinates: None,
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "England",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "London",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((51.5072, 0.1276)),
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "Netherlands",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Amsterdam",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((52.3676, 4.9041)),
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "Germany",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Frankfurt",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((50.1109, 8.6821)),
						child_regions: vec![],
					}],
				},
			],
		},
		DefaultDeploymentRegion {
			name: "North-America",
			cloud_provider: None,
			coordinates: None,
			child_regions: vec![
				DefaultDeploymentRegion {
					name: "Canada",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![DefaultDeploymentRegion {
						name: "Toronto",
						cloud_provider: Some(DeploymentCloudProvider::Digitalocean),
						coordinates: Some((43.6532, 79.3832)),
						child_regions: vec![],
					}],
				},
				DefaultDeploymentRegion {
					name: "USA",
					cloud_provider: None,
					coordinates: None,
					child_regions: vec![
						DefaultDeploymentRegion {
							name: "New-York 1",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((40.7128, 74.0060)),
							child_regions: vec![],
						},
						DefaultDeploymentRegion {
							name: "New-York 2",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((40.7128, 74.0060)),
							child_regions: vec![],
						},
						DefaultDeploymentRegion {
							name: "San Francisco",
							cloud_provider: Some(
								DeploymentCloudProvider::Digitalocean,
							),
							coordinates: Some((37.7749, 122.4194)),
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
	pub child_regions: Vec<DefaultDeploymentRegion>,
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
pub struct Customer {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,

	#[serde(rename = "billing_address[first_name]")]
	pub first_name: Option<String>,

	#[serde(rename = "billing_address[last_name]")]
	pub last_name: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub email: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none")]
	pub phone: Option<String>,

	#[serde(skip_serializing_if = "Option::is_none", flatten)]
	pub address: Option<BillingAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingAddress {
	#[serde(rename = "billing_address[line1]")]
	pub address_line1: String,

	#[serde(
		rename = "billing_address[line2]",
		skip_serializing_if = "Option::is_none"
	)]
	pub address_line2: Option<String>,

	#[serde(
		rename = "billing_address[line3]",
		skip_serializing_if = "Option::is_none"
	)]
	pub address_line3: Option<String>,

	#[serde(rename = "billing_address[city]")]
	pub city: String,

	#[serde(rename = "billing_address[state]")]
	pub state: String,

	#[serde(rename = "billing_address[zip]")]
	pub zip: String,

	#[serde(rename = "billing_address[country]")]
	pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
	pub id: String,
	#[serde(rename = "subscription_items[item_price_id]")]
	pub item_price_id: String,
	#[serde(rename = "subscription_items[quantity]")]
	pub quantity: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionalCreditList {
	pub list: Vec<PromotionalCreditBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionalCreditBalance {
	pub id: String,
	pub customer_id: Uuid,
	pub r#type: String,
	pub amount: i64,
	pub description: String,
	pub credit_type: String,
	pub closing_balance: i64,
	pub created_at: u64,
	pub object: String,
	pub currency_code: String,
}
