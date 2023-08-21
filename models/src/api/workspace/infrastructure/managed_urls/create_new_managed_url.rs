use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::ManagedUrlType;
use crate::{utils::Uuid, ApiRequest};

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
pub struct CreateNewManagedUrlPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNewManagedUrlRequest {
	pub sub_domain: String,
	pub domain_id: Uuid,
	pub path: String,
	#[serde(flatten)]
	pub url_type: ManagedUrlType,
}

impl ApiRequest for CreateNewManagedUrlRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateNewManagedUrlPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateNewManagedUrlResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNewManagedUrlResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{CreateNewManagedUrlRequest, CreateNewManagedUrlResponse};
	use crate::{
		models::workspace::infrastructure::managed_urls::ManagedUrlType,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateNewManagedUrlRequest {
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
			},
			&[
				Token::Map { len: None },
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
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&CreateNewManagedUrlResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateNewManagedUrlResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateNewManagedUrlResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		);
	}
}
