use std::{
	fmt::Display,
	net::{Ipv4Addr, Ipv6Addr},
	str::FromStr,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};

mod add_dns_record;
mod add_domain_to_workspace;
mod delete_dns_record;
mod delete_domain_in_workspace;
mod get_domain_dns_record;
mod get_domain_info_in_workspace;
mod get_domains_for_workspace;
mod is_domain_personal;
mod update_domain_dns_record;
mod verify_domain_in_workspace;

pub use self::{
	add_dns_record::*,
	add_domain_to_workspace::*,
	delete_dns_record::*,
	delete_domain_in_workspace::*,
	get_domain_dns_record::*,
	get_domain_info_in_workspace::*,
	get_domains_for_workspace::*,
	is_domain_personal::*,
	update_domain_dns_record::*,
	verify_domain_in_workspace::*,
};
use crate::utils::{DateTime, Uuid};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
	pub id: Uuid,
	pub name: String,
	pub last_unverified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDomain {
	#[serde(flatten)]
	pub domain: Domain,
	pub is_verified: bool,
	pub nameserver_type: DomainNameserverType,
}

impl WorkspaceDomain {
	pub fn is_ns_internal(&self) -> bool {
		self.nameserver_type.is_internal()
	}

	pub fn is_ns_external(&self) -> bool {
		self.nameserver_type.is_external()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatrControlledDomain {
	pub domain_id: Uuid,
	pub nameserver_type: DomainNameserverType,
	pub zone_identifier: String,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "DOMAIN_NAMESERVER_TYPE", rename_all = "lowercase")]
pub enum DomainNameserverType {
	Internal,
	External,
}

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DomainNameserverType {
	Internal,
	External,
}

impl DomainNameserverType {
	pub fn is_external(&self) -> bool {
		matches!(self, DomainNameserverType::External)
	}

	pub fn is_internal(&self) -> bool {
		matches!(self, DomainNameserverType::Internal)
	}
}

impl Display for DomainNameserverType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Internal => write!(f, "internal"),
			Self::External => write!(f, "external"),
		}
	}
}

impl FromStr for DomainNameserverType {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"internal" => Ok(Self::Internal),
			"external" => Ok(Self::External),
			_ => Err(s),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatrDomainDnsRecord {
	pub id: Uuid,
	pub domain_id: Uuid,
	pub name: String,
	#[serde(flatten)]
	pub r#type: DnsRecordValue,
	pub ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
#[allow(clippy::upper_case_acronyms)]
pub enum DnsRecordValue {
	A { target: Ipv4Addr, proxied: bool },
	MX { priority: u16, target: String },
	TXT { target: String },
	AAAA { target: Ipv6Addr, proxied: bool },
	CNAME { target: String, proxied: bool },
}

impl DnsRecordValue {
	pub fn is_a_record(&self) -> bool {
		matches!(self, DnsRecordValue::A { .. })
	}

	pub fn is_aaaa_record(&self) -> bool {
		matches!(self, DnsRecordValue::AAAA { .. })
	}

	pub fn is_cname_record(&self) -> bool {
		matches!(self, DnsRecordValue::CNAME { .. })
	}

	pub fn is_mx_record(&self) -> bool {
		matches!(self, DnsRecordValue::MX { .. })
	}

	pub fn is_txt_record(&self) -> bool {
		matches!(self, DnsRecordValue::TXT { .. })
	}

	pub fn as_a_record(&self) -> Option<(&Ipv4Addr, bool)> {
		match self {
			DnsRecordValue::A { target, proxied } => Some((target, *proxied)),
			_ => None,
		}
	}

	pub fn as_aaaa_record(&self) -> Option<(&Ipv6Addr, bool)> {
		match self {
			DnsRecordValue::AAAA { target, proxied } => {
				Some((target, *proxied))
			}
			_ => None,
		}
	}

	pub fn as_cname_record(&self) -> Option<(&str, bool)> {
		match self {
			DnsRecordValue::CNAME { target, proxied } => {
				Some((target, *proxied))
			}
			_ => None,
		}
	}

	pub fn as_mx_record(&self) -> Option<(u16, &str)> {
		match self {
			DnsRecordValue::MX { priority, target } => {
				Some((*priority, target))
			}
			_ => None,
		}
	}

	pub fn as_txt_record(&self) -> Option<&str> {
		match self {
			DnsRecordValue::TXT { target } => Some(target),
			_ => None,
		}
	}
}

impl Display for DnsRecordValue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::A { .. } => write!(f, "A"),
			Self::AAAA { .. } => write!(f, "AAAA"),
			Self::CNAME { .. } => write!(f, "CNAME"),
			Self::MX { .. } => write!(f, "MX"),
			Self::TXT { .. } => write!(f, "TXT"),
		}
	}
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Configure, Token};

	use super::{DnsRecordValue, Domain, PatrDomainDnsRecord};
	use crate::utils::{DateTime, Uuid};

	#[test]
	fn assert_domain_type() {
		assert_tokens(
			&Domain {
				id: Uuid::parse_str("2bef18631ded45eb9170dc2266b30567")
					.unwrap(),
				name: "patrtest.patr.cloud".to_string(),
				last_unverified: Some(DateTime::default()),
			},
			&[
				Token::Struct {
					name: "Domain",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2bef18631ded45eb9170dc2266b30567"),
				Token::Str("name"),
				Token::Str("patrtest.patr.cloud"),
				Token::Str("lastUnverified"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_dns_record_type() {
		assert_tokens(
			&PatrDomainDnsRecord {
				id: Uuid::parse_str("2bff18631ded45eb9170dc2266b30577")
					.unwrap(),
				domain_id: Uuid::parse_str("2bff18631ded45eb9170dc2266b30567")
					.unwrap(),
				name: "patrtest.patr.cloud".to_string(),
				r#type: DnsRecordValue::A {
					target: "192.168.1.1".parse().unwrap(),
					proxied: true,
				},
				ttl: 3600,
			}
			.readable(),
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2bff18631ded45eb9170dc2266b30577"),
				Token::Str("domainId"),
				Token::Str("2bff18631ded45eb9170dc2266b30567"),
				Token::Str("name"),
				Token::Str("patrtest.patr.cloud"),
				Token::Str("type"),
				Token::Str("A"),
				Token::Str("target"),
				Token::Str("192.168.1.1"),
				Token::Str("proxied"),
				Token::Bool(true),
				Token::Str("ttl"),
				Token::U32(3600),
				Token::MapEnd,
			],
		)
	}
}
