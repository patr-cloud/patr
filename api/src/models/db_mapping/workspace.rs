use std::{fmt::Display, str::FromStr};

use eve_rs::AsError;
use serde::{Deserialize, Serialize};

use crate::{
	error,
	utils::{constants::ResourceOwnerType, Error},
};

pub struct Workspace {
	pub id: Vec<u8>,
	pub name: String,
	pub super_admin_id: Vec<u8>,
	pub active: bool,
}

pub struct Domain {
	pub id: Vec<u8>,
	pub name: String,
	pub r#type: ResourceOwnerType,
}

pub struct PersonalDomain {
	pub id: Vec<u8>,
	pub name: String,
	pub domain_type: ResourceOwnerType,
}

pub struct WorkspaceDomain {
	pub id: Vec<u8>,
	pub name: String,
	pub domain_type: ResourceOwnerType,
	pub is_verified: bool,
	pub is_patr_controlled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
	pub domain_id: Vec<u8>,
	pub name: String,
	pub a_record: Vec<String>,
	pub aaaa_record: Vec<String>,
	pub cname_record: String,
	pub mx_record: Vec<String>,
	pub text_record: Vec<String>,
	pub ttl: i32,
	pub proxied: bool,
	pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntryPoint {
	pub domain_id: Vec<u8>,
	pub is_verified: bool,
	pub sub_domains: String,
	pub path: String,
	pub deployment_id: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatrControlledDomain {
	pub domain_id: Vec<u8>,
	pub control_status: DomainControlStatus,
	pub zone_identifier: Vec<u8>,
	pub is_verified: bool,
}

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug, PartialEq)]
#[sqlx(type_name = "DOMAIN_CONTROL_STATUS", rename_all = "lowercase")]
pub enum DomainControlStatus {
	Patr,
	User,
}

impl Display for DomainControlStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Patr => write!(f, "patr"),
			Self::User => write!(f, "user"),
		}
	}
}

impl FromStr for DomainControlStatus {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"patr" => Ok(Self::Patr),
			"user" => Ok(Self::User),
			_ => Error::as_result()
				.status(500)
				.body(error!(WRONG_PARAMETERS).to_string()),
		}
	}
}
