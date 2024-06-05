use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use strum::VariantNames;

use crate::prelude::*;

/// A list of all possible resource types in Patr.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum ResourceType {
	/// A workspace is also considered a resource
	Workspace,
	/// A project within a workspace
	Project,
	/// A runner within a workspace
	Runner,
	/// A deployment
	Deployment,
	/// A managed database
	Database,
	/// A static site
	StaticSite,
	/// A container registry repository
	ContainerRegistryRepository,
	/// A secret
	Secret,
	/// A domain added to a workspace
	Domain,
	/// A DNS record within a workspace
	DnsRecord,
	/// A Managed URL for a particular deployment / static site, or otherwise
	ManagedURL,
}

/// A list of all permissions that can be granted on a DNS record.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum DnsRecordPermissions {
	/// Add a DNS record to a domain
	Add,
	/// View a DNS record and it's details in a domain
	View,
	/// Edit a DNS record in a domain
	Edit,
	/// Delete a DNS record in a domain
	Delete,
}

/// A list of all permissions that can be granted on a domain.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum DomainPermissions {
	/// Add a domain to a workspace
	Add,
	/// View a domain and it's details in a workspace
	View,
	/// Verify the domain (regardless of if it's internal or external)
	Verify,
	/// Delete the domain
	Delete,
}

/// A list of all permissions that can be granted on a deployment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum DeploymentPermissions {
	/// Create a deployment on a workspace
	Create,
	/// View a deployment and it's details
	View,
	/// Edit a deployment
	Edit,
	/// Delete a deployment
	Delete,
}

/// A list of all permissions that can be granted on a container registry
/// repository.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum ContainerRegistryRepositoryPermissions {
	/// Create a repository in the container registry
	Create,
	/// View the repository and it's details
	View,
	/// Edit the repository
	Edit,
	/// Delete the repository
	Delete,
}

/// A list of all permissions that can be used for workspace billing stuff.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum BillingPermissions {
	/// View billing information of a workspace
	View,
	/// Edit billing information for a workspace
	Edit,
	/// Make a payment for a workspace using an existing payment method
	MakePayment,
}

/// A list of all permissions that can be granted on a resource.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum Permission {
	/// All permissions related to a domain
	Domain(DomainPermissions),
	/// All permissions related to a DNS record
	DnsRecord(DnsRecordPermissions),
	/// All permissions related to a deployment
	Deployment(DeploymentPermissions),
	/// All permissions related to container registry repositories
	ContainerRegistryRepository(ContainerRegistryRepositoryPermissions),
	/// All permissions for a workspace's billing
	Billing(BillingPermissions),
	/// The user is allowed to change the workspace name and basic stuff
	EditWorkspace,
}

/// Represents the kind of permission that is granted on a workspace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
	pub fn is_super_admin(&self) -> bool {
		matches!(self, WorkspacePermission::SuperAdmin)
	}

	/// Returns true if the user is a member of the workspace.
	pub fn is_member(&self) -> bool {
		matches!(self, WorkspacePermission::Member { .. })
	}

	/// Returns true if the current [`WorkspacePermission`] instance has more or
	/// equal permissions than the other [`WorkspacePermission`] instance.
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
}

/// Represents the data of a resource permission, which type of resource the
/// permission is granted on, and the resources that the permission is granted
/// on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePermissionData {
	/// The type of resource that the permission is granted on.
	pub resource_type: ResourceType,
	/// The resources that the permission is granted on.
	#[serde(flatten)]
	pub resources: ResourcePermissionType,
}

/// Represents the type of permission that is granted on a set of Resource IDs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
	rename_all = "camelCase",
	tag = "permissionType",
	content = "resources"
)]
pub enum ResourcePermissionType {
	/// The user is allowed to access a set of Resource IDs. Any other
	/// Resource IDs are by default not allowed.
	Include(
		/// Set of Resource IDs to allow
		BTreeSet<Uuid>,
	),
	/// The user is not allowed to access a set of Resource IDs. Any other
	/// Resource IDs are by default allowed.
	Exclude(
		/// Set of Resource IDs to not allow
		BTreeSet<Uuid>,
	),
}
