use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::WorkspaceDomain;
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
#[typed_path("/workspace/:workspace_id/domain/:domain_id")]
pub struct GetDomainInfoPath {
	pub workspace_id: Uuid,
	pub domain_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDomainInfoRequest;

impl ApiRequest for GetDomainInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDomainInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetDomainInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDomainInfoResponse {
	#[serde(flatten)]
	pub workspace_domain: WorkspaceDomain,
}

#[cfg(test)]
mod test {

	use serde_test::{assert_tokens, Token};

	use super::{GetDomainInfoRequest, GetDomainInfoResponse};
	use crate::{
		models::workspace::domain::{
			Domain,
			DomainNameserverType,
			WorkspaceDomain,
		},
		utils::{DateTime, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDomainInfoRequest,
			&[Token::UnitStruct {
				name: "GetDomainInfoRequest",
			}],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDomainInfoResponse {
				workspace_domain: WorkspaceDomain {
					domain: Domain {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "test.patr.cloud".to_string(),
						last_unverified: Some(DateTime::default()),
					},
					is_verified: false,
					nameserver_type: DomainNameserverType::Internal,
				},
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("test.patr.cloud"),
				Token::Str("lastUnverified"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isVerified"),
				Token::Bool(false),
				Token::Str("nameserverType"),
				Token::UnitVariant {
					name: "DomainNameserverType",
					variant: "internal",
				},
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDomainInfoResponse {
				workspace_domain: WorkspaceDomain {
					domain: Domain {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "test.patr.cloud".to_string(),
						last_unverified: Some(DateTime::default()),
					},
					is_verified: false,
					nameserver_type: DomainNameserverType::Internal,
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("test.patr.cloud"),
				Token::Str("lastUnverified"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isVerified"),
				Token::Bool(false),
				Token::Str("nameserverType"),
				Token::UnitVariant {
					name: "DomainNameserverType",
					variant: "internal",
				},
				Token::MapEnd,
			],
		)
	}
}
