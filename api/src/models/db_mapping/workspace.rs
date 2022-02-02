use std::{fmt::Display, str::FromStr};

use api_models::{
	models::workspace::domain::DomainNameserverType,
	utils::{ResourceType, Uuid},
};
use eve_rs::AsError;
use serde::{Deserialize, Serialize};

use crate::{error, utils::Error};

pub struct Workspace {
	pub id: Uuid,
	pub name: String,
	pub super_admin_id: Uuid,
	pub active: bool,
}

pub struct Domain {
	pub id: Uuid,
	pub name: String,
	pub r#type: ResourceType,
}

pub struct PersonalDomain {
	pub id: Uuid,
	pub name: String,
	pub domain_type: ResourceType,
}

pub struct WorkspaceDomain {
	pub id: Uuid,
	pub name: String,
	pub domain_type: ResourceType,
	pub is_verified: bool,
	pub nameserver_type: DomainNameserverType,
}

impl WorkspaceDomain {
	pub fn is_ns_external(&self) -> bool {
		self.nameserver_type.is_external()
	}

	pub fn is_ns_internal(&self) -> bool {
		self.nameserver_type.is_internal()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
	pub id: Uuid,
	pub record_identifier: String,
	pub domain_id: Uuid,
	pub name: String,
	pub r#type: DnsRecordType,
	pub value: String,
	pub priority: Option<i32>,
	pub ttl: i64,
	pub proxied: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatrControlledDomain {
	pub domain_id: Uuid,
	pub nameserver_type: DomainNameserverType,
	pub zone_identifier: String,
}

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "DNS_RECORD_TYPE", rename_all = "UPPERCASE")]
#[allow(clippy::upper_case_acronyms)]
pub enum DnsRecordType {
	A,
	AAAA,
	CNAME,
	MX,
	TXT,
}

impl Display for DnsRecordType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::A => write!(f, "A"),
			Self::AAAA => write!(f, "AAAA"),
			Self::CNAME => write!(f, "CNAME"),
			Self::MX => write!(f, "MX"),
			Self::TXT => write!(f, "TXT"),
		}
	}
}

impl FromStr for DnsRecordType {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_uppercase().as_str() {
			"A" => Ok(Self::A),
			"AAAA" => Ok(Self::AAAA),
			"CNAME" => Ok(Self::CNAME),
			"MX" => Ok(Self::MX),
			"TXT" => Ok(Self::TXT),
			_ => Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}
