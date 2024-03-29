use serde::{Deserialize, Serialize};

mod create_managed_url;
mod delete_managed_url;
mod list_managed_url;
mod update_managed_url;
mod verify_configuration;

pub use self::{
	create_managed_url::*,
	delete_managed_url::*,
	list_managed_url::*,
	update_managed_url::*,
	verify_configuration::*,
};
use crate::utils::Uuid;

/// Which field to order the list by for paginated requests
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ListOrderBy {
	/// Order the list by the status of the resource
	Status,
	/// Order the list by the name of the resource
	Name,
	/// Order the list by when the resource was last updated
	LastUpdated,
	/// Order the list by when the resource was created
	#[default]
	Created,
}

/// Managed URL information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUrl {
	/// Subdomain of the URL
	pub sub_domain: String,
	/// Domain ID of the domain stored in Patr in-house database
	pub domain_id: Uuid,
	/// Entire path of the URL
	pub path: String,
	/// Type of URL
	#[serde(flatten)]
	pub url_type: ManagedUrlType,
	/// Verify if the URL is
	pub is_configured: bool,
}

/// Manageg URL types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ManagedUrlType {
	/// URL is pointing to a deployment
	#[serde(rename_all = "camelCase")]
	ProxyDeployment {
		/// Deployment ID of the deployment to point to
		deployment_id: Uuid,
		/// Deployment port of the deployment to point to
		port: u16,
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
