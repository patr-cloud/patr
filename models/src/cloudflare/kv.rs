use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Managed URL types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkerManagedUrlKVValue {
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
