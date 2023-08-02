use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::WorkspaceDomain;
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
#[typed_path("/workspace/:workspace_id/domain")]
pub struct GetDomainsForWorkspacePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDomainsForWorkspaceRequest;

impl ApiRequest for GetDomainsForWorkspaceRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDomainsForWorkspacePath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetDomainsForWorkspaceResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDomainsForWorkspaceResponse {
	pub domains: Vec<WorkspaceDomain>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		GetDomainsForWorkspaceRequest,
		GetDomainsForWorkspaceResponse,
	};
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
			&GetDomainsForWorkspaceRequest,
			&[Token::UnitStruct {
				name: "GetDomainsForWorkspaceRequest",
			}],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDomainsForWorkspaceResponse {
				domains: vec![
					WorkspaceDomain {
						domain: Domain {
							id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30867",
							)
							.unwrap(),
							name: "test.patr.cloud".to_string(),
							last_unverified: Some(DateTime::default()),
						},
						nameserver_type: DomainNameserverType::Internal,
						is_verified: true,
					},
					WorkspaceDomain {
						domain: Domain {
							id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30868",
							)
							.unwrap(),
							name: "test1.patr.cloud".to_string(),
							last_unverified: Some(DateTime::default()),
						},
						nameserver_type: DomainNameserverType::External,
						is_verified: false,
					},
				],
			},
			&[
				Token::Struct {
					name: "GetDomainsForWorkspaceResponse",
					len: 1,
				},
				Token::Str("domains"),
				Token::Seq { len: Some(2) },
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("test.patr.cloud"),
				Token::Str("lastUnverified"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isVerified"),
				Token::Bool(true),
				Token::Str("nameserverType"),
				Token::UnitVariant {
					name: "DomainNameserverType",
					variant: "internal",
				},
				Token::MapEnd,
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("test1.patr.cloud"),
				Token::Str("lastUnverified"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isVerified"),
				Token::Bool(false),
				Token::Str("nameserverType"),
				Token::UnitVariant {
					name: "DomainNameserverType",
					variant: "external",
				},
				Token::MapEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDomainsForWorkspaceResponse {
				domains: vec![
					WorkspaceDomain {
						domain: Domain {
							id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30867",
							)
							.unwrap(),
							name: "test.patr.cloud".to_string(),
							last_unverified: Some(DateTime::default()),
						},
						nameserver_type: DomainNameserverType::Internal,
						is_verified: false,
					},
					WorkspaceDomain {
						domain: Domain {
							id: Uuid::parse_str(
								"2aef18631ded45eb9170dc2166b30868",
							)
							.unwrap(),
							name: "test1.patr.cloud".to_string(),
							last_unverified: Some(DateTime::default()),
						},
						nameserver_type: DomainNameserverType::External,
						is_verified: true,
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("domains"),
				Token::Seq { len: Some(2) },
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
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("test1.patr.cloud"),
				Token::Str("lastUnverified"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("isVerified"),
				Token::Bool(true),
				Token::Str("nameserverType"),
				Token::UnitVariant {
					name: "DomainNameserverType",
					variant: "external",
				},
				Token::MapEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
