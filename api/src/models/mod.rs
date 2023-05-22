pub mod ci;
pub mod cloudflare;
pub mod deployment;
pub mod error;
pub mod rabbitmq;
pub mod rbac;
pub mod region;
pub mod secret;

mod auditlog;
mod auth;
mod docker_registry;
mod email_template;
#[cfg(feature = "sample-data")]
mod sample_data;
mod twilio_sms;

use std::fmt;

use api_models::utils::Uuid;
use serde::{Deserialize, Serialize};

#[cfg(feature = "sample-data")]
pub use self::sample_data::*;
pub use self::{
	auditlog::*,
	auth::*,
	docker_registry::*,
	email_template::*,
	region::*,
	twilio_sms::*,
};

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceType {
	Deployment,
	StaticSite,
	ManagedDatabase,
	ManagedUrl,
	Secret,
	DockerRepository,
	Domain,
	DNSRecord,
	CiRepo,
	Region,
}

impl fmt::Display for ResourceType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			ResourceType::Deployment => write!(f, "Deployment"),
			ResourceType::DockerRepository => write!(f, "Docker repository"),
			ResourceType::StaticSite => write!(f, "Static site"),
			ResourceType::ManagedDatabase => write!(f, "Patr database"),
			ResourceType::ManagedUrl => write!(f, "Managed url"),
			ResourceType::Secret => write!(f, "Secret"),
			ResourceType::Domain => write!(f, "Domain"),
			ResourceType::DNSRecord => write!(f, "DNS record"),
			ResourceType::CiRepo => write!(f, "Ci repo"),
			ResourceType::Region => write!(f, "Deployment region"),
		}
	}
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDeployment {
	pub deployment_name: String,
	pub deployment_id: Uuid,
	pub hours: u64,
	pub instances: u32,
	pub estimated_cost: u32,
	pub ram_count: u32,
	pub cpu_count: u32,
	pub plan: String,
}
