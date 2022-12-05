pub mod routing {
	use std::{collections::HashMap, fmt::Display};

	use api_models::utils::Uuid;
	use serde::{Deserialize, Serialize};

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
	#[serde(tag = "type", rename_all = "camelCase")]
	pub enum ManagedUrlType {
		#[serde(rename_all = "camelCase")]
		ProxyDeployment { deployment_id: Uuid },
		#[serde(rename_all = "camelCase")]
		ProxyStaticSite { static_site_id: Uuid },
		#[serde(rename_all = "camelCase")]
		ProxyUrl { url: String },
		#[serde(rename_all = "camelCase")]
		Redirect { url: String },
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Value(pub HashMap<String, ManagedUrlType>);
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

	// todo: how to handle stopped and deleted case
	#[derive(Debug, Serialize, Deserialize)]
	pub struct Value {
		pub region_id: Uuid,
		pub ports: Vec<u16>,
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

	// todo: how to handle stopped and deleted case
	#[derive(Debug, Serialize, Deserialize)]
	pub struct Value {
		pub upload_id: Uuid,
	}
}
