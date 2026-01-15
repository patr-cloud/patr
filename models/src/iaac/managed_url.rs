use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::{iaac::MaybeExternallySourced, prelude::*};

/// The IaaC definition for a managed URL. This is similar to the
/// [`ManagedUrl`] struct, but with fields optimized for the IaaC file format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct IaacManagedUrl {
	/// The ID of the managed URL that needs to be patched (used to identify
	/// the managed URL if the subdomain or domain has changed)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub id: Option<Uuid>,
	/// The subdomain of the managed URL
	#[serde(alias = "subdomain")]
	pub sub_domain: MaybeExternallySourced<String>,
	/// The domain name (referenced by name, not ID in IaaC)
	pub domain: MaybeExternallySourced<String>,
	/// The path of the URL
	pub path: MaybeExternallySourced<String>,
	/// The type of URL (Deployment, Static Site, Proxy, Redirect)
	#[serde(flatten)]
	pub to: IaacManagedUrlType,
}

impl Hash for IaacManagedUrl {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.map(|id| id.hash(state)).unwrap_or_else(|| {
			self.sub_domain.hash(state);
			self.domain.hash(state);
			self.path.hash(state);
		});
	}
}

/// The IaaC representation of ManagedUrlType, using deployment names instead
/// of IDs for better readability in IaaC files
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "to", rename_all = "snake_case")]
pub enum IaacManagedUrlType {
	/// URL is pointing to a deployment
	#[serde(alias = "deployment", rename_all = "snake_case")]
	ProxyDeployment {
		/// Name of the deployment to point to
		deployment: String,
		/// Deployment port of the deployment to point to
		port: u16,
	},
	/// URL is pointing to a static site
	#[serde(rename_all = "snake_case")]
	ProxyStaticSite {
		/// Name of the static site to point to
		#[serde(alias = "staticSite")]
		static_site: String,
	},
	/// URL is a proxy
	#[serde(alias = "url", rename_all = "snake_case")]
	ProxyUrl {
		/// The URL of the proxy
		url: String,
		/// If the URL is http only
		#[serde(default)]
		http_only: bool,
	},
	/// URL is a redirect to another site
	#[serde(rename_all = "snake_case")]
	Redirect {
		/// The URL to redirect to
		url: String,
		/// If the URL is a permanent redirect
		#[serde(default)]
		permanent_redirect: bool,
		/// If the URL is http only
		#[serde(default)]
		http_only: bool,
	},
}
