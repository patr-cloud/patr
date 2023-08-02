use serde::{Deserialize, Serialize};

mod create_new_managed_url;
mod delete_managed_url;
mod list_managed_urls;
mod update_managed_url;
mod verify_configuration;

pub use self::{
	create_new_managed_url::*,
	delete_managed_url::*,
	list_managed_urls::*,
	update_managed_url::*,
	verify_configuration::*,
};
use crate::utils::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUrl {
	pub id: Uuid,
	pub sub_domain: String,
	pub domain_id: Uuid,
	pub path: String,
	#[serde(flatten)]
	pub url_type: ManagedUrlType,
	pub is_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ManagedUrlType {
	#[serde(rename_all = "camelCase")]
	ProxyDeployment { deployment_id: Uuid, port: u16 },
	#[serde(rename_all = "camelCase")]
	ProxyStaticSite { static_site_id: Uuid },
	#[serde(rename_all = "camelCase")]
	ProxyUrl { url: String, http_only: bool },
	#[serde(rename_all = "camelCase")]
	Redirect {
		url: String,
		permanent_redirect: bool,
		http_only: bool,
	},
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ManagedUrl, ManagedUrlType};
	use crate::utils::Uuid;

	#[test]
	fn assert_managed_url_type_types() {
		assert_tokens(
			&ManagedUrlType::ProxyDeployment {
				deployment_id: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30867",
				)
				.unwrap(),
				port: 8080,
			},
			&[
				Token::Struct {
					name: "ManagedUrlType",
					len: 3,
				},
				Token::Str("type"),
				Token::Str("proxyDeployment"),
				Token::Str("deploymentId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("port"),
				Token::U16(8080),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_managed_url_types() {
		assert_tokens(
			&ManagedUrl {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				sub_domain: "test".to_string(),
				domain_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				path: "/".to_string(),
				url_type: ManagedUrlType::ProxyDeployment {
					deployment_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30867",
					)
					.unwrap(),
					port: 8080,
				},
				is_configured: true,
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("subDomain"),
				Token::Str("test"),
				Token::Str("domainId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("path"),
				Token::Str("/"),
				Token::Str("type"),
				Token::Str("proxyDeployment"),
				Token::Str("deploymentId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("port"),
				Token::U16(8080),
				Token::Str("isConfigured"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}
}
