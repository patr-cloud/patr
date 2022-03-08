use std::{collections::HashMap, fmt::Display, str::FromStr};

use api_models::utils::Uuid;
use eve_rs::AsError;
use once_cell::sync::OnceCell;
use serde::Deserialize;

use super::db_mapping::DeploymentCloudProvider;
use crate::{
	error,
	utils::{get_current_time_millis, Error},
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
	pub value: f64,
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
			Interval::Hour => get_current_time_millis() - 3600000,
			Interval::Day => get_current_time_millis() - 86400000,
			Interval::Week => get_current_time_millis() - 604800000,
			Interval::Month => get_current_time_millis() - 2628000000,
			Interval::Year => get_current_time_millis() - 31556952000,
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
	type Err = String;

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
			_ => Err(s),
		}
	}
}
