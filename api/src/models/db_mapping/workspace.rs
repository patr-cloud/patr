use serde::{Deserialize, Serialize};

use crate::utils::constants::ResourceOwnerType;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DnsRecord {
	pub deployment_id: Vec<u8>,
	pub domain_id: Vec<u8>,
	pub name: String,
	pub a_record: Vec<String>,
	pub aaaa_record: Vec<String>,
	pub cname_record: String,
	pub mx_record: Vec<String>,
	pub text_record: Vec<String>,
	pub content: String,
	pub ttl: i32,
	pub proxied: bool,
}
