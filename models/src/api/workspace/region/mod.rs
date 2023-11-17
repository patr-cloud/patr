use std::{fmt::Display, str::FromStr};
use serde::{Deserialize, Serialize};

mod add_region_to_workspace;
mod check_region_status;
mod delete_region;
mod get_region_info;
mod list_regions_for_workspace;

pub use self::{
	add_region_to_workspace::*,
	check_region_status::*,
	delete_region::*,
};

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "INFRASTRUCTURE_CLOUD_PROVIDER", rename_all = "lowercase")]
pub enum InfrastructureCloudProvider {
	Digitalocean,
	Other,
}

/// Cloud providers
#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InfrastructureCloudProvider {
	/// DigitalOcean
	Digitalocean,
	/// Other cloud providers
	Other,
}

impl Display for InfrastructureCloudProvider {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Digitalocean => write!(f, "digitalocean"),
			Self::Other => write!(f, "other"),
		}
	}
}

impl FromStr for InfrastructureCloudProvider {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"digitalocean" => Ok(Self::Digitalocean),
			"other" => Ok(Self::Other),
			_ => Err(format!("Invalid cloud provider: {s}")),
		}
	}
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "REGION_STATUS", rename_all = "snake_case")]
pub enum RegionStatus {
	Creating,
	Active,
	Errored,
	Deleted,
	Disconnected,
	ComingSoon,
}

/// Region status
#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RegionStatus {
	/// Region is creating
	Creating,
	/// Region is active
	Active,
	/// Region has errored
	Errored,
	/// Region is deleted
	Deleted,
	/// Region has been disconnected
	Disconnected,
	/// Region is currently not supported
	ComingSoon,
}

/// Region type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RegionType {
	/// Patr owned region
	PatrOwned,
	#[serde(rename = "byoc")]
	/// BYOC region
	BYOC,
}

/// Region information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Region {
	/// Name of the region
	pub name: String,
	/// Cloud sertvice provider
	pub cloud_provider: InfrastructureCloudProvider,
	/// Status of the region
	pub status: RegionStatus,
	/// Region type
	#[serde(flatten)]
	pub r#type: RegionType,
}
