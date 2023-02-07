pub mod routing {
	use std::{collections::HashMap, fmt::Display};

	use api_models::utils::Uuid;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
	#[serde(tag = "type", rename_all = "camelCase")]
	pub enum UrlType {
		#[serde(rename_all = "camelCase")]
		ProxyDeployment { deployment_id: Uuid },
		#[serde(rename_all = "camelCase")]
		ProxyStaticSite { static_site_id: Uuid },
		#[serde(rename_all = "camelCase")]
		ProxyUrl { url: String },
		#[serde(rename_all = "camelCase")]
		Redirect { url: String },
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

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Value(pub HashMap<String, UrlType>);
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
	pub enum Status {
		Created,
		Stopped,
		Deleted,
		#[serde(rename_all = "camelCase")]
		Running {
			ports: Vec<u16>,
		},
	}

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(rename_all = "camelCase")]
	pub struct Value {
		pub region_id: Uuid,
		pub status: Status,
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
		Running {
			upload_id: Uuid,
		},
	}
}

pub mod region {
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
	pub struct Value {
		pub host: String,
	}
}