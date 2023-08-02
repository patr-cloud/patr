use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::utils::Uuid;

mod add_region_to_workspace;
mod check_region_status;
mod delete_region;
mod get_region_info;
mod list_regions_for_workspace;

pub use self::{
	add_region_to_workspace::*,
	check_region_status::*,
	delete_region::*,
	get_region_info::*,
	list_regions_for_workspace::*,
};

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "INFRASTRUCTURE_CLOUD_PROVIDER", rename_all = "lowercase")]
pub enum InfrastructureCloudProvider {
	Digitalocean,
	Other,
}

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InfrastructureCloudProvider {
	Digitalocean,
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

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RegionStatus {
	Creating,
	Active,
	Errored,
	Deleted,
	Disconnected,
	ComingSoon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RegionType {
	PatrOwned,
	#[serde(rename = "byoc")]
	BYOC,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Region {
	pub id: Uuid,
	pub name: String,
	pub cloud_provider: InfrastructureCloudProvider,
	pub status: RegionStatus,
	#[serde(flatten)]
	pub r#type: RegionType,
}
