use std::{collections::BTreeMap, marker::ConstParamTy};

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumMessage, EnumString, VariantNames};

use crate::prelude::*;

/// The list of all permissions that can be granted on a Resource.
mod permissions;
/// Represents the type of permission that is granted on a set of Resource IDs.
mod resource_permission_type;

pub use self::{permissions::*, resource_permission_type::*};

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
	VariantNames,
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
}

/// Represents the kind of permission that is granted on a workspace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkspacePermission {
	/// The user is the super admin of the workspace.
	SuperAdmin,
	/// The user is a member of the workspace.
	Member {
		/// List of Permission IDs and the type of permission that is granted.
		#[serde(flatten)]
		permissions: BTreeMap<Uuid, ResourcePermissionType>,
	},
}

impl WorkspacePermission {
	/// Returns true if the user is a super admin of the workspace.
	#[must_use]
	pub fn is_super_admin(&self) -> bool {
		matches!(self, WorkspacePermission::SuperAdmin)
	}

	/// Returns true if the user is a member of the workspace.
	#[must_use]
	pub fn is_member(&self) -> bool {
		matches!(self, WorkspacePermission::Member { .. })
	}

	/// Returns true if the current [`WorkspacePermission`] instance has more or
	/// equal permissions than the other [`WorkspacePermission`] instance.
	#[must_use]
	pub fn is_superset_of(&self, other: &WorkspacePermission) -> bool {
		match (self, other) {
			// If you're a super admin, you have all permissions. So go ahead, regardless of what
			// you're requesting, you're allowed.
			(Self::SuperAdmin, _) => true,
			// If you're a member, and you're asking for super admin permissions,
			// that's disallowed.
			(Self::Member { .. }, Self::SuperAdmin) => false,
			// If you're a member, you are requesting member permissions, then we need to check
			// deeper.
			(
				Self::Member {
					permissions: self_permissions,
				},
				Self::Member {
					permissions: other_permissions,
				},
			) => other_permissions
				.iter()
				.all(|(permission_id, other_resources)| {
					let Some(self_resources) = self_permissions.get(permission_id) else {
						return false;
					};
					match (self_resources, other_resources) {
						(
							ResourcePermissionType::Include(self_resources),
							ResourcePermissionType::Include(other_resources),
						) => self_resources.is_superset(other_resources),
						(
							ResourcePermissionType::Include(_),
							ResourcePermissionType::Exclude(_),
						) => {
							// If the current permission is to include a set of resources, and
							// the other permission is to exclude a set of resources, then the
							// current permission is not a subset of the other permission.
							//
							// Why? Simple example:
							// If the list of resources are [1, 2, 3, 4, 5], and the include
							// permission has a list of resources [1, 2, 3], and the exclude
							// permission has a list of resources [4], then the include permission
							// is not a subset of the exclude permission. In this case, the include
							// permission has access to resources 1, 2, 3, but the exclude
							// permission has access to resources 1, 2, 3, 5.
							//
							// The only way that the include permission would be a subset of the
							// exclude permission is if the exclude permission had a list of all
							// resources that are an exact inverse of the include permission. But
							// that also might not always work. Even if the exclude permission has a
							// list of all resources that are an exact inverse of the include
							// permission, when the user creates a new resource, the new resource
							// would be accessible by the exclude permission, but not the include
							// permission.
							//
							// So yeah, we're straight up rejecting this

							false
						}
						(
							ResourcePermissionType::Exclude(self_resources),
							ResourcePermissionType::Include(other_resources),
						) => {
							// Okay see, the user has an exclude permission, and the other
							// permission is to include a set of resources. This is a bit
							// tricky.
							//
							// If the user has an exclude permission, then the user is
							// allowed to access all resources except the ones that are in
							// the exclude list. So if the other permission is to include a
							// set of resources, then any resource is allowed, as long as it
							// is not in the exclude list.
							self_resources.is_disjoint(other_resources)
						}
						(
							ResourcePermissionType::Exclude(self_resources),
							ResourcePermissionType::Exclude(other_resources),
						) => {
							// This is tough to explain, but I'm gonna try.
							// Your current permissions are on all resources except a few. The other
							// permissions are also on all resources except a few. If the resources
							// that other permissions are excluding is bigger than the current one,
							// then that's cool. Cuz as long as others aren't accessing the
							// resources in the current list, they are free to exclude other
							// resources as well.
							self_resources.is_subset(other_resources)
						}
					}
				}),
		}
	}

	/// Returns true if the user has the specified permission on the given
	/// resource.
	#[must_use]
	pub fn has_resource_permission(&self, resource_id: Uuid, permission_id: Uuid) -> bool {
		match self {
			Self::SuperAdmin => true,
			Self::Member { permissions } => permissions
				.get(&permission_id)
				.map_or(false, |resource_permissions| {
					resource_permissions.has_resource(&resource_id)
				}),
		}
	}
}
