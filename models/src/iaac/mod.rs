mod database;
mod deployment;

use either::Either;
use serde::{Deserialize, Serialize};

pub use self::{database::*, deployment::*};
use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Resource {
	#[serde(flatten)]
	pub data: IaacResource,
	pub depends_on: OneOrMore<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum IaacResource {
	Deployment(IaacDeployment),
	Database,
	StaticSite,
	ManagedUrl,
	Domain,
	DockerRepository,
	Secret,
}

impl IaacResource {
	pub fn get_resource_type(&self) -> IaacResourceType {
		match self {
			Self::Deployment(_) => IaacResourceType::Deployment,
			Self::Database => IaacResourceType::Database,
			Self::StaticSite => IaacResourceType::StaticSite,
			Self::ManagedUrl => IaacResourceType::ManagedUrl,
			Self::Domain => IaacResourceType::Domain,
			Self::DockerRepository => IaacResourceType::DockerRepository,
			Self::Secret => IaacResourceType::Secret,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IaacResourceType {
	Deployment,
	Database,
	StaticSite,
	ManagedUrl,
	Domain,
	DockerRepository,
	Secret,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
	pub resource: IaacResourceType,
	pub identifier: Either<Uuid, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged, rename_all = "camelCase", deny_unknown_fields)]
pub enum MaybeExternallySourced<T> {
	Value(T),
	#[serde(rename_all = "snake_case")]
	FromEnvironment {
		#[serde(alias = "env")]
		from_env: String,
	},
	#[serde(rename_all = "snake_case")]
	FromResource {
		#[serde(alias = "env")]
		from_resource: String,
	},
}
