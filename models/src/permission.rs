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
	/// All permissions related to DNS records for a domain
	DnsRecord(DnsRecordPermissions),
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
	/// Make a payment for a workspace
	MakePayment,
}

/// A list of all permissions that can be granted on a resource.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum Permission {
	/// All permissions related to a domain
	Domain(DomainPermissions),
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
