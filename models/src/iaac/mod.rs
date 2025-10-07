use std::{
	fmt::Display,
	hash::{Hash, Hasher},
	str::FromStr,
};

use either::Either;
use serde::{Deserialize, Serialize};

pub use self::{database::*, deployment::*, error::*};
use crate::{prelude::*, rbac::ResourceType};

/// All database related IaaC structs and functions.
mod database;
/// All deployment related IaaC structs and functions.
mod deployment;
/// All Iaac related error types.
mod error;

/// Any resource that can be defined in an Iaac file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IaacResource {
	/// The resource data, which can be a deployment, database, etc.
	#[serde(flatten)]
	pub data: IaacResourceData,
	/// What the resource depends on, e.g. a database or another deployment.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub depends_on: Option<OneOrMore<Dependency>>,
}

impl IaacResource {
	/// Returns the type of the resource, e.g. `deployment`, `database`, etc.
	pub fn resource_type(&self) -> ResourceType {
		self.data.get_resource_type()
	}

	/// Returns the name of the resource, which is used to identify it in Iaac
	/// files.
	pub fn name(&self) -> &MaybeExternallySourced<String> {
		self.data.name()
	}

	/// Returns the dependencies of the resource, which can be a single
	/// dependency or multiple dependencies. If there are no dependencies, it
	/// returns an empty slice.
	pub fn dependencies(&self) -> &[Dependency] {
		match self.depends_on.as_ref() {
			Some(OneOrMore::One(dependency)) => std::slice::from_ref(dependency),
			Some(OneOrMore::Multiple(dependencies)) => dependencies.as_slice(),
			None => &[],
		}
	}
}

impl Hash for IaacResource {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.data.get_resource_type().hash(state);
		self.data.name().hash(state);
	}
}

/// The Iaac resource that is defined in the Iaac file. This is a particular
/// resource that can be deployed, such as a deployment, database, static site,
/// managed URL, domain, Docker repository, or secret.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum IaacResourceData {
	/// A deployment resource, which is a containerized application.
	Deployment(IaacDeployment),
}

impl IaacResourceData {
	/// Returns the type of the resource, e.g. `deployment`, `database`, etc.
	pub fn get_resource_type(&self) -> ResourceType {
		match self {
			Self::Deployment(_) => ResourceType::Deployment,
		}
	}

	/// Returns the name of the resource, which is used to identify it in Iaac
	/// files.
	pub fn name(&self) -> &MaybeExternallySourced<String> {
		match self {
			Self::Deployment(deployment) => &deployment.name,
		}
	}
}

/// A dependency of a resource, which can either be a UUID or a name. This is
/// used to define what a resource depends on, e.g. a database or another
/// deployment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
	/// The type of the resource that this dependency is for, e.g. `deployment`,
	/// `database`, etc. This is only required if the identifier is a name AND
	/// multiple resources with that name exist.
	pub resource: Option<ResourceType>,
	/// The identifier of the resource that this dependency is for. This can be
	/// either a UUID or a name. If it is a name, it must be unique within the
	/// workspace.
	pub identifier: Either<Uuid, String>,
}

/// A helper type to parse a value that can either be a raw value or a reference
/// from an external source (such as an environment variable). This is used
/// to allow for more flexibility in Iaac files, where values can be defined
/// as either raw strings or references to external sources.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged, rename_all = "camelCase", deny_unknown_fields)]
pub enum MaybeExternallySourced<T> {
	/// A raw value. This is used when the value is a simple string or number.
	Value(T),
	/// A value that is sourced from an environment variable.
	#[serde(rename_all = "snake_case", alias = "from_env")]
	FromEnvironment {
		/// The name of the environment variable to source the value from.
		#[serde(alias = "env")]
		from_env: String,
	},
	// Maybe tomorrow we can add more types of external sources, such as
	// from another resource's value, etc
}

impl<T> MaybeExternallySourced<T>
where
	T: FromStr,
	<T as FromStr>::Err: Display,
{
	/// Returns the value of the `MaybeExternallySourced` type, either the raw
	/// value or the value sourced from an environment variable.
	pub fn resolve_value(self) -> Result<T, IaacError> {
		match self {
			Self::Value(value) => Ok(value),
			Self::FromEnvironment { from_env } => std::env::var(&from_env)
				.map(|value| T::from_str(&value))
				.map_err(|_| IaacError::EnvironmentVariableNotFound(from_env))?
				.map_err(|err| IaacError::EnvironmentVariableParseError(err.to_string())),
		}
	}
}

impl<T> Default for MaybeExternallySourced<T>
where
	T: Default,
{
	fn default() -> Self {
		Self::Value(T::default())
	}
}

impl<T> From<T> for MaybeExternallySourced<T> {
	fn from(value: T) -> Self {
		Self::Value(value)
	}
}
