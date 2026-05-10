use std::marker::ConstParamTy;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumMessage, EnumString, VariantArray};

/// A list of all possible resource types in Patr.
#[derive(
	Eq,
	Ord,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	PartialOrd,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantArray,
	ConstParamTy,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum ResourceType {
	/// A workspace, which is also considered a resource
	Workspace,
	/// A project within a workspace. A project can be used to group resources,
	/// and provide users permissions only on those specific resources,
	Project,
	/// A runner within a workspace. A runner is used to run deployments,
	/// databases, static sites, secrets, domains, etc.
	Runner,
	/// A deployment within a workspace. A deployment is a running instance of a
	/// container image. It can be scaled horizontally, and can be configured to
	/// deploy on push.
	Deployment,
	/// A container registry repository within a workspace. A container registry
	/// repository is a collection of container images that can be deployed to
	/// a deployment, which will be run on a runner.
	ContainerRegistryRepository,
	/// A domain added to a workspace. A domain can be used to access
	/// deployments. It can be verified, and used as the host for a managed URL.
	Domain,
	/// A Managed URL for a particular deployment, or otherwise. A
	/// managed URL is a URL that is managed by Patr, and is accessible over the
	/// internet. It is automatically updated when the deployment is updated.
	ManagedURL,
	/// A role within a workspace. A role is a collection of permissions that
	/// can be granted to a user. It is associated with a workspace, and can be
	/// assigned to users within that workspace. A role can be used to grant
	/// permissions on resources within a workspace.
	Role,
}

impl ResourceType {
	/// Returns a list of all resource types.
	#[must_use]
	pub fn list_all() -> Vec<Self> {
		Self::VARIANTS.to_vec()
	}

	/// Returns the description of the resource type, as per the documentation
	/// of the resource type.
	///
	/// # Panics
	/// Panics if the resource type does not have a documentation. This should
	/// not happen, as all resource types should have a documentation.
	#[must_use]
	pub fn description(&self) -> String {
		self.get_documentation()
			.expect("Documentation not found")
			.to_string()
	}
}
