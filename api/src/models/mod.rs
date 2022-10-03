pub mod billing;
pub mod ci;
pub mod deployment;
pub mod error;
pub mod rabbitmq;
pub mod rbac;

mod access_token_data;
mod auditlog;
mod docker_registry;
mod email_template;
#[cfg(feature = "sample-data")]
mod sample_data;
mod twilio_sms;

use std::fmt;

use serde::{Deserialize, Serialize};

#[cfg(feature = "sample-data")]
pub use self::sample_data::*;
pub use self::{
	access_token_data::*,
	auditlog::*,
	docker_registry::*,
	email_template::*,
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
}

impl fmt::Display for ResourceType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			ResourceType::Deployment => write!(f, "Deployment"),
			ResourceType::DockerRepository => write!(f, "Docker repository"),
			ResourceType::StaticSite => write!(f, "Static site"),
			ResourceType::ManagedDatabase => write!(f, "Managed database"),
			ResourceType::ManagedUrl => write!(f, "Managed url"),
			ResourceType::Secret => write!(f, "Secret"),
			ResourceType::Domain => write!(f, "Domain"),
			ResourceType::DNSRecord => write!(f, "DNS record"),
		}
	}
}
