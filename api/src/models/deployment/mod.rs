use std::{collections::HashMap, fmt::Display, str::FromStr};

use api_models::{
	models::workspace::infrastructure::DeploymentCloudProvider,
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use eve_rs::AsError;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::{
	error,
	utils::{get_current_time, Error},
};

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

#[derive(Debug, Clone)]
pub enum Interval {
	Hour,
	Day,
	Week,
	Month,
	Year,
}

impl Interval {
	pub fn as_u64(&self) -> u64 {
		match self {
			Interval::Hour => get_current_time().as_secs() - 3600,
			Interval::Day => get_current_time().as_secs() - 86400,
			Interval::Week => get_current_time().as_secs() - 604800,
			Interval::Month => get_current_time().as_secs() - 2628000,
			Interval::Year => get_current_time().as_secs() - 31556952,
		}
	}
}

impl FromStr for Interval {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"hour" | "hr" | "h" => Ok(Self::Hour),
			"day" | "d" => Ok(Self::Day),
			"week" | "w" => Ok(Self::Week),
			"month" | "mnth" | "m" => Ok(Self::Month),
			"year" | "yr" | "y" => Ok(Self::Year),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}

#[derive(Debug, Clone)]
pub enum Step {
	OneMinute,
	TwoMinutes,
	FiveMinutes,
	TenMinutes,
	FifteenMinutes,
	ThirtyMinutes,
	OneHour,
}

impl Display for Step {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::OneMinute => write!(f, "1m"),
			Self::TwoMinutes => write!(f, "2m"),
			Self::FiveMinutes => write!(f, "5m"),
			Self::TenMinutes => write!(f, "10m"),
			Self::FifteenMinutes => write!(f, "15m"),
			Self::ThirtyMinutes => write!(f, "30m"),
			Self::OneHour => write!(f, "1h"),
		}
	}
}

impl FromStr for Step {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"1m" => Ok(Self::OneMinute),
			"2m" => Ok(Self::TwoMinutes),
			"5m" => Ok(Self::FiveMinutes),
			"10m" => Ok(Self::TenMinutes),
			"15m" => Ok(Self::FifteenMinutes),
			"30m" => Ok(Self::ThirtyMinutes),
			"1h" => Ok(Self::OneHour),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
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

	#[serde(
		rename = "billing_address[first_name]",
		skip_serializing_if = "Option::is_none"
	)]
	pub first_name: Option<String>,

	#[serde(
		rename = "billing_address[last_name]",
		skip_serializing_if = "Option::is_none"
	)]
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
pub struct Subscription {
	pub id: String,
	#[serde(rename = "subscription_items[item_price_id][0]")]
	pub item_price_id: String,
	#[serde(rename = "subscription_items[quantity][0]")]
	pub quantity: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubscription {
	#[serde(
		rename = "subscription_items[item_price_id][0]",
		skip_serializing_if = "Option::is_none"
	)]
	pub item_price_id: Option<String>,
	#[serde(
		rename = "subscription_items[quantity][0]",
		skip_serializing_if = "Option::is_none"
	)]
	pub quantity: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionalCreditList {
	pub list: Vec<PromotionalCredit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionalCredit {
	pub promotional_credit: PromotionalCreditBalance,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSourceList {
	pub list: Vec<PaymentSources>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSources {
	pub payment_source: PaymentSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSource {
	pub id: String,
	pub updated_at: u64,
	pub deleted: bool,
	pub object: String,
	pub customer_id: String,
	pub r#type: String,
	pub reference_id: String,
	pub status: String,
	pub gateway: String,
	pub gateway_account_id: String,
	pub created_at: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub card: Option<Card>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Card {
	pub first_name: Option<String>,
	pub last_name: Option<String>,
	pub iin: String,
	pub last4: String,
	pub funding_type: String,
	pub expiry_month: u8,
	pub expiry_year: u16,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub billing_addr1: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub billing_addre2: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub billing_city: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub billing_state: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub billing_zip: Option<String>,
	pub masked_number: String,
	pub object: String,
	pub brand: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionList {
	pub list: Vec<Subscriptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscriptions {
	pub subscription: SubscriptionResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
	pub id: String,
	pub billing_period: u32,
	pub billing_period_unit: String,
	pub customer_id: String,
	pub status: String,
	pub current_term_start: u64,
	pub current_term_end: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub next_billing_at: Option<u64>,
	pub created_at: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub started_at: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub activated_at: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cancelled_at: Option<u64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub updated_at: Option<u64>,
	pub has_scheduled_changes: bool,
	pub channel: String,
	pub object: String,
	pub currency_code: String,
	pub subscription_items: Vec<SubscriptionItem>,
	pub due_invoices_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionItem {
	pub item_price_id: String,
	pub item_type: String,
	pub quantity: u16,
	pub unit_price: u64,
	pub amount: u64,
	pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdatePaymentMethod {
	pub hosted_page: HostedPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HostedPage {
	pub id: String,
	pub r#type: String,
	pub url: String,
	pub state: String,
	pub embed: bool,
	pub created_at: u64,
	pub expires_at: u64,
	pub object: String,
	pub updated_at: u64,
	pub resource_version: u64,
}
