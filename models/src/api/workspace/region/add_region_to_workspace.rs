use crate::{prelude::*, utils::BearerToken};
use std::fmt;
use serde::{Serialize, Deserialize};

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
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
		/// Node gorup name
		node_name: String,
		/// Node size
		node_size_slug: String,
	},
	#[serde(rename_all = "camelCase")]
	/// Add region from a kubconfig file
	KubeConfig {
		/// The Kubeconfig file to connect a cluster
		config_file: String,
	},
}

macros::declare_api_endpoint!(
	/// Route to add region to a workspace
	AddRegionToWorkspace,
	POST "/workspace/:workspace_id/region" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// Name of the region
		pub name: String,
		/// The region data
		pub data: AddRegionToWorkspaceData,
	},
	response = {
		/// The ID of the created region
		#[serde(flatten)]
		pub region_id: WithId<()>
	}
);
