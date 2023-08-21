use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

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
#[typed_path(
	"/workspace/:workspace_id/domain/:domain_id/dns-record/:record_id"
)]
pub struct UpdateDomainDnsRecordPath {
	pub workspace_id: Uuid,
	pub domain_id: Uuid,
	pub record_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDomainDnsRecordRequest {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ttl: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub target: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub priority: Option<u16>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub proxied: Option<bool>,
}

impl ApiRequest for UpdateDomainDnsRecordRequest {
	const METHOD: Method = Method::PATCH;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateDomainDnsRecordPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::UpdateDomainDnsRecordRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateDomainDnsRecordRequest {
				ttl: Some(3600),
				target: Some("192.168.1.1".to_string()),
				priority: None,
				proxied: Some(false),
			},
			&[
				Token::Struct {
					name: "UpdateDomainDnsRecordRequest",
					len: 3,
				},
				Token::Str("ttl"),
				Token::Some,
				Token::U32(3600),
				Token::Str("target"),
				Token::Some,
				Token::Str("192.168.1.1"),
				Token::Str("proxied"),
				Token::Some,
				Token::Bool(false),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_empty_request_types() {
		assert_tokens(
			&UpdateDomainDnsRecordRequest {
				ttl: None,
				target: None,
				priority: None,
				proxied: None,
			},
			&[
				Token::Struct {
					name: "UpdateDomainDnsRecordRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<UpdateDomainDnsRecordRequest as ApiRequest>::Response,
		>(());
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(()),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}
}
