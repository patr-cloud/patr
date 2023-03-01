pub mod routing {
	use std::fmt::Display;

	use api_models::utils::Uuid;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
	#[serde(tag = "type", rename_all = "camelCase")]
	pub enum UrlType {
		#[serde(rename_all = "camelCase")]
		ProxyDeployment { deployment_id: Uuid, port: u16 },
		#[serde(rename_all = "camelCase")]
		ProxyStaticSite { static_site_id: Uuid },
		#[serde(rename_all = "camelCase")]
		ProxyUrl { url: String },
		#[serde(rename_all = "camelCase")]
		Redirect { url: String, permanent: bool },
	}

	#[derive(Debug)]
	pub struct Key {
		pub sub_domain: String,
		pub domain: String,
	}

	impl Display for Key {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "{}.{}", self.sub_domain, self.domain)
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
	#[serde(rename_all = "camelCase")]
	pub struct RouteType {
		pub path: String,
		#[serde(flatten)]
		pub url_type: UrlType,
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Value(pub Vec<RouteType>);
}

pub mod deployment {
	use std::fmt::Display;

	use api_models::utils::Uuid;
	use serde::{Deserialize, Serialize};

	pub struct Key(pub Uuid);

	impl Display for Key {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "{}", self.0)
		}
	}

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "camelCase")]
	pub enum Value {
		Created,
		Stopped,
		Deleted,
		#[serde(rename_all = "camelCase")]
		Running {
			region_id: Uuid,
			ports: Vec<u16>,
		},
	}
}

pub mod static_site {
	use std::fmt::Display;

	use api_models::utils::Uuid;
	use serde::{Deserialize, Serialize};

	pub struct Key(pub Uuid);

	impl Display for Key {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "{}", self.0)
		}
	}

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "camelCase")]
	pub enum Value {
		Created,
		Stopped,
		Deleted,
		#[serde(rename_all = "camelCase")]
		Serving(Uuid),
	}
}
