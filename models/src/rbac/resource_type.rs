use std::marker::ConstParamTy;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumMessage, EnumString, VariantArray};
use ts_rs::TS;

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
	TS,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
// Exported under a distinct name so it does not collide with the existing
// `ResourceType` binding (the `{ name, description }` metadata struct from
// `list_all_resource_types`).
#[ts(export, rename = "ResourceTypeName")]
pub enum ResourceType {
	/// A workspace, which is also considered a resource
	Workspace,
	/// A project within a workspace. A project can be used to group resources,
	/// and provide users permissions only on those specific resources,
	Project,
	/// A runner within a workspace. A runner is used to run deployments,
	/// secrets, domains, etc.
	Runner,
	/// A deployment within a workspace. A deployment is a running instance of a
	/// container image. It can be scaled horizontally, and can be configured to
	/// deploy on push.
	Deployment,
	/// A volume within a workspace. A volume is a persistent storage that can
	/// be attached to a deployment. It can be used to store data that needs to
	/// persist across deployments.
	Volume,
	/// A container registry repository within a workspace. A container registry
	/// repository is a collection of container images that can be deployed to
	/// a deployment, which will be run on a runner.
	ContainerRegistryRepository,
	/// A secret within a workspace. A secret is a key-value pair that can be
	/// used in deployments. It is encrypted at rest and in transit. It can be
	/// rotated, and is only accessible by the deployment that it is associated
	/// with.
	Secret,
	/// A domain added to a workspace. A domain can be used to access
	/// deployments. It can be verified, and used for managed URLs.
	Domain,
	/// A Managed URL for a particular deployment, or otherwise. A managed URL
	/// is a URL that is managed by Patr, and is accessible over the internet.
	/// It can be used to access deployments, etc. It is managed by Patr, and
	/// is automatically updated when the deployment is updated.
	ManagedURL,
	/// A role within a workspace. A role is a collection of permissions that
	/// can be granted to a user. It is associated with a workspace, and can be
	/// assigned to users within that workspace. A role can be used to grant
	/// permissions on resources within a workspace.
	Role,
	/// A service account within a workspace. A service account is a non-human
	/// identity that can be used to authenticate runners and other automated
	/// processes. It has a single token and can be assigned roles within its
	/// workspace.
	ServiceAccount,
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
