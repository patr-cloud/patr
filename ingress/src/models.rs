use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IngressKVData {
	#[serde(rename_all = "camelCase")]
	Redirect {
		to: String,
		permanent_redirect: bool,
		http_only: bool,
	},
	#[serde(rename_all = "camelCase")]
	Proxy { to: String, http_only: bool },
	#[serde(rename_all = "camelCase")]
	StaticSite {
		static_site_id: String,
		upload_id: String,
	},
	#[serde(rename_all = "camelCase")]
	Deployment {
		deployment_id: String,
		port: u16,
		region: String,
	},
}

impl IngressKVData {
	/// Check if the data is a redirect
	pub fn is_redirect(&self) -> bool {
		matches!(self, IngressKVData::Redirect { .. })
	}
}
