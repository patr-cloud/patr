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
	/// databases, static sites, secrets, domains, etc.
	Runner,
	/// A deployment within a workspace. A deployment is a running instance of a
	/// container image. It can be scaled horizontally, and can be configured to
	/// deploy on push.
	Deployment,
	/// A volume within a workspace. A volume is a persistent storage that can
	/// be attached to a deployment. It can be used to store data that needs to
	/// persist across deployments.
	Volume,
	/// A database within a workspace. A database is a running instance of a
	/// database server, such as `MySQL`, `PostgreSQL`, etc. It can be scaled
	/// and persists data across deployments. It can also be shelled into for
	/// debugging purposes.
	Database,
	/// A static site within a workspace. A static site is a collection of files
	/// that are served over HTTP. Static sites are automatically deployed and
	/// are accessible via a managed URL.
	StaticSite,
	/// A container registry repository within a workspace. A container registry
	/// repository is a collection of container images that can be deployed to
	/// a deployment, which will be run on a runner.
	ContainerRegistryRepository,
	/// A secret within a workspace. A secret is a key-value pair that can be
	/// used in deployments, databases, etc. It is encrypted at rest and in
	/// transit. It can be rotated, and is only accessible by the deployment /
	/// database that it is associated with.
	Secret,
	/// A domain added to a workspace. A domain can be used to access
	/// deployments and static sites. It can be verified, and can have DNS
	/// records added to it.
	Domain,
	/// A DNS record within a workspace. A DNS record is a record that points a
	/// domain to an IP address. It can be added to a domain, and can be used to
	/// point a domain to a deployment or static site. A DNS record can be used
	/// to point a domain to a deployment or static site.
	DnsRecord,
	/// A Managed URL for a particular deployment / static site, or otherwise. A
	/// managed URL is a URL that is managed by Patr, and is accessible over the
	/// internet. It can be used to access deployments, static sites, etc. It is
	/// managed by Patr, and is automatically updated when the deployment /
	/// static site is updated.
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
