use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::DnsRecordValue;
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
#[typed_path("/workspace/:workspace_id/domain/:domain_id/dns-record")]
pub struct AddDnsRecordPath {
	pub workspace_id: Uuid,
	pub domain_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddDnsRecordRequest {
	pub name: String,
	#[serde(flatten)]
	pub r#type: DnsRecordValue,
	pub ttl: u32,
}

impl ApiRequest for AddDnsRecordRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = AddDnsRecordPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = AddDnsRecordResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddDnsRecordResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Configure, Token};

	use super::{AddDnsRecordRequest, AddDnsRecordResponse};
	use crate::{
		models::workspace::domain::DnsRecordValue,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_for_a_record_types() {
		assert_tokens(
			&AddDnsRecordRequest {
				name: "patrtest.patr.cloud".to_string(),
				r#type: DnsRecordValue::A {
					target: "192.168.1.1".parse().unwrap(),
					proxied: false,
				},
				ttl: 3600,
			}
			.readable(),
			&[
				Token::Map { len: None },
				Token::Str("name"),
				Token::Str("patrtest.patr.cloud"),
				Token::Str("type"),
				Token::Str("A"),
				Token::Str("target"),
				Token::Str("192.168.1.1"),
				Token::Str("proxied"),
				Token::Bool(false),
				Token::Str("ttl"),
				Token::U32(3600),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_request_for_mx_record_types() {
		assert_tokens(
			&AddDnsRecordRequest {
				name: "patrtest.patr.cloud".to_string(),
				r#type: DnsRecordValue::MX {
					target: "mail.patr.cloud".to_string(),
					priority: 20,
				},
				ttl: 3600,
			},
			&[
				Token::Map { len: None },
				Token::Str("name"),
				Token::Str("patrtest.patr.cloud"),
				Token::Str("type"),
				Token::Str("MX"),
				Token::Str("priority"),
				Token::U16(20),
				Token::Str("target"),
				Token::Str("mail.patr.cloud"),
				Token::Str("ttl"),
				Token::U32(3600),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&AddDnsRecordResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "AddDnsRecordResponse",
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
			&ApiResponse::success(AddDnsRecordResponse {
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
