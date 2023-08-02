use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	models::workspace::infrastructure::managed_urls::ManagedUrl,
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
#[typed_path("/workspace/:workspace_id/infrastructure/static-site/:static_site_id/managed-urls")]
pub struct ListLinkedURLsPath {
	pub workspace_id: Uuid,
	pub static_site_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListLinkedURLsRequest {}

impl ApiRequest for ListLinkedURLsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListLinkedURLsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListLinkedURLsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListLinkedURLsResponse {
	pub urls: Vec<ManagedUrl>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListLinkedURLsRequest, ListLinkedURLsResponse};
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
			&ListLinkedURLsRequest {},
			&[
				Token::Struct {
					name: "ListLinkedURLsRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListLinkedURLsResponse {
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
						url_type: ManagedUrlType::ProxyStaticSite {
							static_site_id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30867",
							)
							.unwrap(),
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
						url_type: ManagedUrlType::Redirect {
							url: "test.vicara.co".to_string(),
							permanent_redirect: false,
							http_only: false,
						},
						is_configured: false,
					},
				],
			},
			&[
				Token::Struct {
					name: "ListLinkedURLsResponse",
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
				Token::Str("proxyStaticSite"),
				Token::Str("staticSiteId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
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
				Token::Str("redirect"),
				Token::Str("url"),
				Token::Str("test.vicara.co"),
				Token::Str("permanentRedirect"),
				Token::Bool(false),
				Token::Str("httpOnly"),
				Token::Bool(false),
				Token::Str("isConfigured"),
				Token::Bool(false),
				Token::MapEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListLinkedURLsResponse {
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
						url_type: ManagedUrlType::ProxyStaticSite {
							static_site_id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30867",
							)
							.unwrap(),
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
						url_type: ManagedUrlType::Redirect {
							url: "test.vicara.co".to_string(),
							permanent_redirect: false,
							http_only: false,
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
				Token::Str("proxyStaticSite"),
				Token::Str("staticSiteId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
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
				Token::Str("redirect"),
				Token::Str("url"),
				Token::Str("test.vicara.co"),
				Token::Str("permanentRedirect"),
				Token::Bool(false),
				Token::Str("httpOnly"),
				Token::Bool(false),
				Token::Str("isConfigured"),
				Token::Bool(false),
				Token::MapEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
