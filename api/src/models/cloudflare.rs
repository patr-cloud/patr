use std::{collections::HashMap, fmt::Display};

use api_models::utils::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct CfKey {
	pub sub_domain: String,
	pub domain: String,
}

impl Display for CfKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}.{}", self.sub_domain, self.domain)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ManagedUrlType {
	#[serde(rename_all = "camelCase")]
	ProxyDeployment { deployment_id: Uuid, port: u16 },
	#[serde(rename_all = "camelCase")]
	ProxyStaticSite { static_site_id: Uuid },
	#[serde(rename_all = "camelCase")]
	ProxyUrl { url: String },
	#[serde(rename_all = "camelCase")]
	Redirect { url: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CfValue(pub HashMap<String, ManagedUrlType>);
