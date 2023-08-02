use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::ManagedUrl;
use crate::{
	utils::{Paginated, Uuid},
	ApiRequest,
};

#[derive(
	Eq,
	Ord,
	Hash,
	Debug,
	Clone,
	Default,
	TypedPath,
	PartialEq,
	Serialize,
	PartialOrd,
	Deserialize,
)]
#[typed_path("/workspace/:workspace_id/infrastructure/managed-url")]
pub struct ListManagedUrlsPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListManagedUrlsRequest {}

impl ApiRequest for ListManagedUrlsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListManagedUrlsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListManagedUrlsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListManagedUrlsResponse {
	pub urls: Vec<ManagedUrl>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListManagedUrlsRequest, ListManagedUrlsResponse};
	use crate::{
		models::workspace::infrastructure::managed_urls::{
			ManagedUrl,
			ManagedUrlType,
		},
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListManagedUrlsRequest {},
			&[
				Token::Struct {
					name: "ListManagedUrlsRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListManagedUrlsResponse {
				urls: vec![
					ManagedUrl {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						sub_domain: "test".to_string(),
						domain_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
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
					ManagedUrl {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						sub_domain: "test2".to_string(),
						domain_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						path: "/".to_string(),
						url_type: ManagedUrlType::ProxyStaticSite {
							static_site_id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30867",
							)
							.unwrap(),
						},
						is_configured: false,
					},
				],
			},
			&[
				Token::Struct {
					name: "ListManagedUrlsResponse",
					len: 1,
				},
				Token::Str("urls"),
				Token::Seq { len: Some(2) },
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
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("subDomain"),
				Token::Str("test2"),
				Token::Str("domainId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("path"),
				Token::Str("/"),
				Token::Str("type"),
				Token::Str("proxyStaticSite"),
				Token::Str("staticSiteId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("isConfigured"),
				Token::Bool(false),
				Token::MapEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListManagedUrlsResponse {
				urls: vec![
					ManagedUrl {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						sub_domain: "test".to_string(),
						domain_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
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
					ManagedUrl {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						sub_domain: "test2".to_string(),
						domain_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						path: "/".to_string(),
						url_type: ManagedUrlType::ProxyStaticSite {
							static_site_id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30867",
							)
							.unwrap(),
						},
						is_configured: false,
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("urls"),
				Token::Seq { len: Some(2) },
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
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("subDomain"),
				Token::Str("test2"),
				Token::Str("domainId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("path"),
				Token::Str("/"),
				Token::Str("type"),
				Token::Str("proxyStaticSite"),
				Token::Str("staticSiteId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("isConfigured"),
				Token::Bool(false),
				Token::MapEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
