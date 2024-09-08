use std::{
	fmt::Display,
	net::{Ipv4Addr, Ipv6Addr},
	str::FromStr,
};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// The endpoint to add a DNS record to a domain
mod add_dns_record;
/// The endpoint to add a domain to a workspace
mod add_domain_to_workspace;
/// The endpoint to delete a DNS record from a domain
mod delete_dns_record;
/// The endpoint to delete a domain from a workspace
mod delete_domain_in_workspace;
/// The endpoint to list all DNS records of a domain
mod get_domain_dns_record;
/// The endpoint to get the domain information in a workspace
mod get_domain_info_in_workspace;
/// The endpoint to get all the domains in a workspace
mod get_domains_for_workspace;
/// The endpoint to check if a domain is personal
mod is_domain_personal;
/// The endpoint to update a DNS record of a domain
mod update_domain_dns_record;
/// The endpoint to verify a domain in a workspace
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

/// The domain metadata information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
	/// The name of the domain
	pub name: String,
	/// Last verified time of the domain
	pub last_unverified: Option<OffsetDateTime>,
}

/// The domain information in a workspace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDomain {
	/// The domain metadata
	#[serde(flatten)]
	pub domain: Domain,
	/// Whether or not the domain is verified
	pub is_verified: bool,
	/// The domain nameserver type
	pub nameserver_type: DomainNameserverType,
}

impl WorkspaceDomain {
	/// To check if the nameserver is internal
	pub fn is_ns_internal(&self) -> bool {
		self.nameserver_type.is_internal()
	}

	/// To check if the nameserver is external
	pub fn is_ns_external(&self) -> bool {
		self.nameserver_type.is_external()
	}
}

/// Patr domain information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatrControlledDomain {
	/// The domain ID
	pub domain_id: Uuid,
	/// The domain nameserver type
	pub nameserver_type: DomainNameserverType,
	/// The domain zone identifier
	pub zone_identifier: String,
}

/// The DNS record type of a domain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
#[serde(tag = "type")]
pub enum DnsRecordValue {
	/// A record for IPv4 addresses
	A {
		/// The target address
		target: Ipv4Addr,
		/// If the DNS record should be proxied or not
		proxied: bool,
	},
	/// MX record for mail servers
	MX {
		/// The priority
		priority: u16,
		/// The target address
		target: String,
	},
	/// TXT record for text information
	TXT {
		/// The target address
		target: String,
	},
	/// AAAA record for IPv6 addresses
	AAAA {
		/// The target address
		target: Ipv6Addr,
		/// If the DNS record should be proxied or not
		proxied: bool,
	},
	/// CNAME record for aliases
	CNAME {
		/// The target address
		target: String,
		/// If the DNS record should be proxied or not
		proxied: bool,
	},
}

impl DnsRecordValue {
	/// To check if the record is of type A
	pub fn is_a_record(&self) -> bool {
		matches!(self, DnsRecordValue::A { .. })
	}

	/// To check if the record is of type AAAA
	pub fn is_aaaa_record(&self) -> bool {
		matches!(self, DnsRecordValue::AAAA { .. })
	}

	/// To check if the record is of type CNAME
	pub fn is_cname_record(&self) -> bool {
		matches!(self, DnsRecordValue::CNAME { .. })
	}

	/// To check if the record is of type MX
	pub fn is_mx_record(&self) -> bool {
		matches!(self, DnsRecordValue::MX { .. })
	}

	/// To check if the record is of type TXT
	pub fn is_txt_record(&self) -> bool {
		matches!(self, DnsRecordValue::TXT { .. })
	}

	/// To return as of type some
	pub fn as_a_record(&self) -> Option<(&Ipv4Addr, bool)> {
		match self {
			DnsRecordValue::A { target, proxied } => Some((target, *proxied)),
			_ => None,
		}
	}

	/// To return as of type some
	pub fn as_aaaa_record(&self) -> Option<(&Ipv6Addr, bool)> {
		match self {
			DnsRecordValue::AAAA { target, proxied } => Some((target, *proxied)),
			_ => None,
		}
	}

	/// To return as of type some
	pub fn as_cname_record(&self) -> Option<(&str, bool)> {
		match self {
			DnsRecordValue::CNAME { target, proxied } => Some((target, *proxied)),
			_ => None,
		}
	}

	/// To return as of type some
	pub fn as_mx_record(&self) -> Option<(u16, &str)> {
		match self {
			DnsRecordValue::MX { priority, target } => Some((*priority, target)),
			_ => None,
		}
	}

	/// To return as of type some
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

/// Type of domain nameserver
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
	not(target_arch = "wasm32"),
	sqlx(type_name = "DOMAIN_NAMESERVER_TYPE", rename_all = "lowercase")
)]
pub enum DomainNameserverType {
	/// Internal
	Internal,
	/// External
	External,
}

impl DomainNameserverType {
	/// To check if external
	pub fn is_external(&self) -> bool {
		matches!(self, DomainNameserverType::External)
	}

	/// To check if internal
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

/// The DNS record information of patr domain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PatrDomainDnsRecord {
	/// The domain ID
	pub domain_id: Uuid,
	/// The domain name
	pub name: String,
	/// The domain type
	#[serde(flatten)]
	pub r#type: DnsRecordValue,
	/// The time to live
	pub ttl: u32,
}
