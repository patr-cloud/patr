use std::{
	fmt::{self, Display, Formatter},
	str::FromStr,
};

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
	get_region_info::*,
	list_regions_for_workspace::*,
};

/// Cloud providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
	not(target_arch = "wasm32"),
	sqlx(type_name = "INFRASTRUCTURE_CLOUD_PROVIDER", rename_all = "lowercase")
)]
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

/// Region status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
	not(target_arch = "wasm32"),
	sqlx(type_name = "REGION_STATUS", rename_all = "snake_case")
)]
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

/// All the DigitalOcean regions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DigitaloceanRegion {
	/// Amsterdam, the Netherlands
	AMS2,
	/// Amsterdam, the Netherlands
	AMS3,
	/// Bangalore, India
	BLR1,
	/// Frankfurt, Germany
	FRA1,
	/// London, United Kingdom
	LON1,
	/// New York City, United States
	NYC1,
	/// New York City, United States
	NYC2,
	/// New York City, United States
	NYC3,
	/// San Francisco, United States
	SFO1,
	/// San Francisco, United States
	SFO2,
	/// San Francisco, United States
	SFO3,
	/// Singapore
	SGP1,
	/// Toronto, Canada
	TOR1,
}

impl fmt::Display for DigitaloceanRegion {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		match self {
			DigitaloceanRegion::AMS2 => write!(f, "ams2"),
			DigitaloceanRegion::AMS3 => write!(f, "ams3"),
			DigitaloceanRegion::BLR1 => write!(f, "blr1"),
			DigitaloceanRegion::FRA1 => write!(f, "fra1"),
			DigitaloceanRegion::LON1 => write!(f, "lon1"),
			DigitaloceanRegion::NYC1 => write!(f, "nyc1"),
			DigitaloceanRegion::NYC2 => write!(f, "nyc2"),
			DigitaloceanRegion::NYC3 => write!(f, "nyc3"),
			DigitaloceanRegion::SFO1 => write!(f, "sfo1"),
			DigitaloceanRegion::SFO2 => write!(f, "sfo2"),
			DigitaloceanRegion::SFO3 => write!(f, "sfo3"),
			DigitaloceanRegion::SGP1 => write!(f, "sgp1"),
			DigitaloceanRegion::TOR1 => write!(f, "tor1"),
		}
	}
}

/// Struct containing inforamtion to add a new region
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AddRegionToWorkspaceData {
	/// Add region of DigitalOcean
	#[serde(rename_all = "camelCase")]
	Digitalocean {
		/// Cluster name
		cluster_name: String,
		/// Region
		region: DigitaloceanRegion,
		/// API Token to connect to the cluster
		api_token: String,
		/// Minimum nodes to maintain
		min_nodes: u16,
		/// Maximum nodes to maintain
		max_nodes: u16,
		/// Whether to auto scale during peak usage
		auto_scale: bool,
		/// Node group name
		node_name: String,
		/// Node size
		node_size_slug: String,
	},
	#[serde(rename_all = "camelCase")]
	/// Add region from a kubeconfig file
	KubeConfig {
		/// The kubeconfig file to connect a cluster
		config_file: String,
	},
}
