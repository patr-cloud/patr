use std::{fmt::Display, str::FromStr};

use api_models::utils::Uuid;
use eve_rs::AsError;
use serde::{Deserialize, Serialize};

use crate::{
	error,
	utils::{constants::ResourceOwnerType, Error},
};

pub struct Workspace {
	pub id: Uuid,
	pub name: String,
	pub super_admin_id: Uuid,
	pub active: bool,
}

pub struct Domain {
	pub id: Uuid,
	pub name: String,
	pub r#type: ResourceOwnerType,
}

pub struct PersonalDomain {
	pub id: Uuid,
	pub name: String,
	pub domain_type: ResourceOwnerType,
}

pub struct WorkspaceDomain {
	pub id: Uuid,
	pub name: String,
	pub domain_type: ResourceOwnerType,
	pub is_verified: bool,
	pub control_status: DomainControlStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
	pub id: Uuid,
	pub domain_id: Uuid,
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
pub struct PatrControlledDomain {
	pub domain_id: Uuid,
	pub control_status: DomainControlStatus,
	pub zone_identifier: String,
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
