use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::PatrDomainDnsRecord;
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
#[typed_path("/workspace/:workspace_id/domain/:domain_id/dns-record")]
pub struct GetDomainDnsRecordsPath {
	pub workspace_id: Uuid,
	pub domain_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDomainDnsRecordsRequest;

impl ApiRequest for GetDomainDnsRecordsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDomainDnsRecordsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = GetDomainDnsRecordsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDomainDnsRecordsResponse {
	pub records: Vec<PatrDomainDnsRecord>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Configure, Token};

	use super::{GetDomainDnsRecordsRequest, GetDomainDnsRecordsResponse};
	use crate::{
		models::workspace::domain::{DnsRecordValue, PatrDomainDnsRecord},
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDomainDnsRecordsRequest,
			&[Token::UnitStruct {
				name: "GetDomainDnsRecordsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDomainDnsRecordsResponse {
				records: vec![
					PatrDomainDnsRecord {
						id: Uuid::parse_str("3aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						domain_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						name: "patrtest.patr.cloud".to_string(),
						r#type: DnsRecordValue::MX {
							target: "mail.patr.cloud".to_string(),
							priority: 10,
						},
						ttl: 3600,
					},
					PatrDomainDnsRecord {
						id: Uuid::parse_str("4aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						domain_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2266b30567",
						)
						.unwrap(),
						name: "patrtest.patr.cloud".to_string(),
						r#type: DnsRecordValue::A {
							target: "192.168.1.1".parse().unwrap(),
							proxied: false,
						},
						ttl: 3600,
					},
					PatrDomainDnsRecord {
						id: Uuid::parse_str("5aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						domain_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30567",
						)
						.unwrap(),
						name: "patrtest.patr.cloud".to_string(),
						r#type: DnsRecordValue::AAAA {
							target: "2001:0db8:85a3:0000:0000:8a2e:0370:7334"
								.parse()
								.unwrap(),
							proxied: true,
						},
						ttl: 3600,
					},
				],
			}
			.readable(),
			&[
				Token::Struct {
					name: "GetDomainDnsRecordsResponse",
					len: 1,
				},
				Token::Str("records"),
				Token::Seq { len: Some(3) },
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("3aef18631ded45eb9170dc2166b30867"),
				Token::Str("domainId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("patrtest.patr.cloud"),
				Token::Str("type"),
				Token::Str("MX"),
				Token::Str("priority"),
				Token::U16(10),
				Token::Str("target"),
				Token::Str("mail.patr.cloud"),
				Token::Str("ttl"),
				Token::U32(3600),
				Token::MapEnd,
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("4aef18631ded45eb9170dc2166b30867"),
				Token::Str("domainId"),
				Token::Str("2aef18631ded45eb9170dc2266b30567"),
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
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("5aef18631ded45eb9170dc2166b30867"),
				Token::Str("domainId"),
				Token::Str("2aef18631ded45eb9170dc2166b30567"),
				Token::Str("name"),
				Token::Str("patrtest.patr.cloud"),
				Token::Str("type"),
				Token::Str("AAAA"),
				Token::Str("target"),
				Token::Str("2001:db8:85a3::8a2e:370:7334"),
				Token::Str("proxied"),
				Token::Bool(true),
				Token::Str("ttl"),
				Token::U32(3600),
				Token::MapEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDomainDnsRecordsResponse {
				records: vec![PatrDomainDnsRecord {
					id: Uuid::parse_str("3aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					domain_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30867",
					)
					.unwrap(),
					name: "patrtest.patr.cloud".to_string(),
					r#type: DnsRecordValue::MX {
						target: "mail.patr.cloud".to_string(),
						priority: 10,
					},
					ttl: 3600,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("records"),
				Token::Seq { len: Some(1) },
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("3aef18631ded45eb9170dc2166b30867"),
				Token::Str("domainId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("patrtest.patr.cloud"),
				Token::Str("type"),
				Token::Str("MX"),
				Token::Str("priority"),
				Token::U16(10),
				Token::Str("target"),
				Token::Str("mail.patr.cloud"),
				Token::Str("ttl"),
				Token::U32(3600),
				Token::MapEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
