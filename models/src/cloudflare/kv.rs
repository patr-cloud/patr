use serde::{Deserialize, Serialize};

use crate::{api::workspace::deployment::DeploymentStatus, prelude::*};

/// Managed URL types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ManagedUrlKVData {
	/// URL is pointing to a deployment
	#[serde(rename_all = "camelCase")]
	ProxyDeployment {
		/// Deployment ID of the deployment to point to
		deployment_id: Uuid,
		/// Deployment port of the deployment to point to
		port: u16,
		/// The runner that is running the deployment
		runner_id: Uuid,
	},
	/// URL is pointing to a static site
	#[serde(rename_all = "camelCase")]
	ProxyStaticSite {
		/// Static site ID of the static site to point to
		static_site_id: Uuid,
		/// The upload ID of the static site
		upload_id: Uuid,
	},
	/// URL is a proxy
	#[serde(rename_all = "camelCase")]
	ProxyUrl {
		/// The URL of the proxy
		url: String,
		/// If the URL is a http only
		http_only: bool,
	},
	/// URL is a redirect to another site
	#[serde(rename_all = "camelCase")]
	Redirect {
		/// The URL
		url: String,
		/// If the URL is a permanent redirect
		permanent_redirect: bool,
		/// If the URL is a http only
		http_only: bool,
	},
}

impl ManagedUrlKVData {
	/// Check if the managed URL is a redirect
	pub fn is_redirect(&self) -> bool {
		matches!(self, ManagedUrlKVData::Redirect { .. })
	}
}

/// Deployment KV Data stored in Cloudflare KV
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum InternalKVData {
	/// The URL is pointing a deployment
	#[serde(rename_all = "camelCase")]
	Deployment {
		/// The ports of the deployment whose data is being stored
		ports: Vec<u16>,
		/// The runner ID running the deployment
		runner_id: Uuid,
		/// The status of the deployment
		status: DeploymentStatus,
	},
	/// The URL is pointing to a runner
	#[serde(rename_all = "camelCase")]
	Runner,
}
