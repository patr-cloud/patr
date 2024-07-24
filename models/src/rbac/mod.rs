use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

use macros::RecursiveEnumIter;
use serde::{Deserialize, Serialize};
use strum::{
	Display,
	EnumDiscriminants,
	EnumIter,
	EnumMessage,
	EnumString,
	IntoEnumIterator,
	VariantNames,
};

use crate::prelude::*;

/// A list of all possible resource types in Patr.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
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
	/// databases, StaticSites, Secrets, Domains, etc.
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
	/// database server, such as MySQL, PostgreSQL, etc. It can be scaled
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

/// A list of all permissions that can be granted on a Database.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum DatabasePermission {
	/// This permission allows the user to create a new Database in a workspace.
	Create,
	/// This permission allows the user to view the details of an existing
	/// database in a workspace.
	View,
	/// This permission allows the user to edit a database in a workspace, but
	/// not delete it or create a new one.
	Edit,
	/// This permission allows the user to delete a database, but not add a new
	/// one or edit an existing one.
	Delete,
	/// This permission allows the user to create backups of the database, but
	/// not restore them on the same instance.
	Backup,
	/// This permission allows the user to restore a backup of the database, but
	/// not create a new backup.
	Restore,
}

/// A list of all permissions that can be granted on a DNS record.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum DnsRecordPermission {
	/// This permission allows the user to add a DNS record to a domain.
	Add,
	/// This permission allows the user to view the already existing DNS
	/// records in a domain.
	View,
	/// This permission allows the user to edit a DNS record in a domain, but
	/// not delete it or create a new one.
	Edit,
	/// This permission allows the user to delete a DNS record from a domain,
	/// but not add a new one or edit an existing one.
	Delete,
}

/// A list of all permissions that can be granted on a domain.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum DomainPermission {
	/// This permission allows the user to add a domain to a workspace, but not
	/// view it, edit it, or delete it. This permission is useful for users or
	/// API tokens that need to add a domain to a workspace, but not do
	/// anything else with it.
	Add,
	/// This permission allows the user to view the domain and it's details,
	/// but cannot modify it in any way. This permission is useful for users or
	/// API tokens that need to only view the domain.
	View,
	/// This permission allows the user to verify the validity of the domain,
	/// but cannot edit it, delete it, or add DNS records to it. This permission
	/// is useful for users or API tokens that need to verify the domain, but
	/// not do anything else with it.
	Verify,
	/// This permission allows the user to only delete the domain.
	Delete,
}

/// A list of all permissions that can be granted on a Managed URL.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum ManagedURLPermission {
	/// This permission allows the user to add a Managed URL to a workspace, but
	/// not view it, edit it, or delete it. This permission is useful for users
	/// or API tokens that need to add a Managed URL to a workspace, but not do
	/// anything else with it.
	Add,
	/// This permission allows the user to view the Managed URL and it's
	/// details, but cannot modify it in any way. This permission is useful for
	/// users or API tokens that need to only view the Managed URL.
	View,
	/// This permission allows the user to verify the validity of the Managed
	/// URL, but cannot edit it, delete it. This permission is useful for users
	/// or API tokens that need to verify the Managed URL, but not do anything
	/// else with it.
	Verify,
	/// This permission allows the user to edit the Managed URL, but not delete
	/// it. The user will only be able to edit the Managed URL, with no other
	/// updates allowed.
	Edit,
	/// This permission allows the user to only delete the Managed URL
	Delete,
}

/// A list of all permissions that can be granted on a Runner.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum RunnerPermission {
	/// This permission allows the user to create a new runner in a workspace.
	/// The user will be able to create a new runner, but not view, edit, or
	/// delete it. This permission is useful for users or API tokens that need
	/// to create a runner, but not do anything else with it.
	Create,
	/// This permission allows the user to only view the runner and it's
	/// details.
	View,
	/// This permission allows the user to only edit the runner, but not delete
	/// it.
	Edit,
	/// This permission allows the user to delete the runner, but not view it or
	/// edit it. This permission is useful for users or API tokens that need to
	/// only delete runners.
	Delete,
	/// This permission allows the user to regenerate the runner token, but not
	/// view it, edit it, or delete it. This permission is useful for users or
	/// API tokens that need to only regenerate the runner token.
	RegenerateToken,
}

/// A list of all permissions that can be granted on a deployment
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum DeploymentPermission {
	/// This permission allows the user to create a new deployment in a
	/// workspace.
	Create,
	/// This permission allows the user to only view the deployment and it's
	/// details.
	View,
	/// This permission allows the user to edit the deployment, but not delete
	/// it or create a new one.
	Edit,
	/// This permission allows the user to delete the deployment, but not create
	/// a new one, view it, or edit it. This permission is useful for users or
	/// API tokens that need to only delete deployments by their ID.
	Delete,
	/// This permission allows the user to start the deployment, but not edit
	/// it. The user will only be able to start the deployment, with no other
	/// updates allowed.
	Start,
	/// This permission allows the user to stop the deployment, but not edit it.
	/// The user will only be able to stop the deployment with no other updates
	/// allowed.
	Stop,
}

/// A list of all permissions that can be granted on a container registry
/// repository.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum ContainerRegistryRepositoryPermission {
	/// This permission allows the user to create a new repository in the
	/// container registry. The user will be able to create a new repository,
	/// but not view, edit, or delete it.
	Create,
	/// This permission allows the user to view the repository and it's details,
	/// but not edit it, delete it, or create a new one.
	View,
	/// This permission allows the user to edit the repository, but not delete
	/// it or create a new one.
	Edit,
	/// This permission allows the user to delete the repository, but not create
	/// a new one, view it, or edit it. This permission is useful for users or
	/// API tokens that need to only delete repositories by their ID.
	Delete,
	/// This permission allows the user to push an image to the repository, but
	/// not view it, edit it, or delete it. This permission is useful for users
	/// or API tokens that need to only push images to repositories.
	Push,
	/// This permission allows the user to pull an image from the repository,
	/// but not view it, edit it, or delete it. This permission is useful for
	/// users or API tokens that need to only pull images from repositories.
	Pull,
	/// This permission allows the user to delete an image from the repository,
	/// but not view it, edit it, or push or pull images from it. This
	/// permission allows the user / API token to only delete images that have
	/// been pushed, instead of deleting the whole repository.
	DeleteImage,
}

/// A list of all permissions that can be granted on a static site
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum StaticSitePermission {
	/// This permission allows the user to create a new static site in the
	/// workspace. The user will be able to create a new site, but not view,
	/// edit, or delete it.
	Create,
	/// This permission allows the user to only view the static site and it's
	/// details. The user will not be able to edit the site, delete it, or
	/// create a new one.
	View,
	/// This permission allows the user to edit the static site, but not delete
	/// it or create a new one. The user will only be able to edit the site,
	/// with no other updates allowed.
	Edit,
	/// This permission allows the user to delete the static site, but not
	/// create a new one, view it, or edit it. This permission is useful for
	/// users or API tokens that need to only delete sites by their ID.
	Delete,
	/// This permission allows the user to upload a new website file to the
	/// static site, but not view it, edit it, or delete it. This permission is
	/// useful for users or API tokens that need to only upload files to sites.
	Upload,
	/// This permission allows the user to start the static site, but not edit
	/// it. The user will only be able to start the static site, with no other
	/// updates allowed.
	Start,
	/// This permission allows the user to stop the static site, but not edit
	/// it. The user will only be able to stop the static site with no other
	/// updates allowed.
	Stop,
}

/// A list of all permissions that can be used for a secret
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum SecretPermission {
	/// This permission allows the user to create a new secret in a workspace.
	Create,
	/// This permission allows the user to view the secret and it's details, but
	/// not edit it, delete it, or create a new one.
	View,
	/// This permission allows the user to edit the secret, but not delete it or
	/// create a new one.
	Edit,
	/// This permission allows the user to delete the secret, but not create a
	/// new one, view it, or edit it. This permission is useful for users or API
	/// tokens that need to only delete secrets by their ID.
	Delete,
}

/// A list of all permissions that can be used for workspace billing stuff.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum BillingPermission {
	/// This permission allows the user to view the billing information of a
	/// workspace, such as the payment method, the billing address, bill due,
	/// etc.
	View,
	/// This permission allows the user to edit the billing information of a
	/// workspace, but not view it.
	Edit,
	/// This permission allows the user to make a payment for a workspace, but
	/// cannot change the payment method, view the billing information, or edit
	/// the billing information.
	MakePayment,
}

/// A list of all permissions that can be granted on a volume.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	EnumIter,
	PartialEq,
	Serialize,
	EnumString,
	EnumMessage,
	Deserialize,
	VariantNames,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum VolumePermission {
	/// This permission allows the user to create a new volume in a workspace.
	Create,
	/// This permission allows the user to view the volume and it's details, but
	/// not edit it, delete it, or create a new one.
	View,
	/// This permission allows the user to edit the volume, but not delete it or
	/// create a new one.
	Edit,
	/// This permission allows the user to delete the volume, but not create a
	/// new one, view it, or edit it. This permission is useful for users or API
	/// tokens that need to only delete volumes by their ID.
	Delete,
}

/// A list of all permissions that can be granted on a resource.
#[derive(
	Eq,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	PartialEq,
	Serialize,
	Deserialize,
	EnumMessage,
	VariantNames,
	RecursiveEnumIter,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub enum Permission {
	/// All permissions related to a domains
	#[strum(to_string = "domain::{0}")]
	Domain(DomainPermission),
	/// All permissions related to a DNS records
	#[strum(to_string = "dnsRecord::{0}")]
	DnsRecord(DnsRecordPermission),
	/// All permissions related to a deployments
	#[strum(to_string = "deployment::{0}")]
	Deployment(DeploymentPermission),
	/// All permissions related to volumes
	#[strum(to_string = "volume::{0}")]
	Volume(VolumePermission),
	/// All permissions related to container registry repositories
	#[strum(to_string = "containerRegistryRepository::{0}")]
	ContainerRegistryRepository(ContainerRegistryRepositoryPermission),
	/// All permissions for a workspace's billing
	#[strum(to_string = "billing::{0}")]
	Billing(BillingPermission),
	/// All permissions for a Managed URL
	#[strum(to_string = "managedURL::{0}")]
	ManagedURL(ManagedURLPermission),
	/// All permissions for a Runner
	#[strum(to_string = "runner::{0}")]
	Runner(RunnerPermission),
	/// All permissions for a database
	#[strum(to_string = "database::{0}")]
	Database(DatabasePermission),
	/// All static site permissions
	#[strum(to_string = "staticSite::{0}")]
	StaticSite(StaticSitePermission),
	/// All secret permissions
	#[strum(to_string = "secret::{0}")]
	Secret(SecretPermission),
	/// View all roles in a workspace
	ViewRoles,
	/// Edit roles in a workspace. This permission allows the user to edit
	/// roles, which includes adding permissions to roles, removing permissions
	/// from roles, and changing the name and description of roles. This is a
	/// powerful permission, and should be granted with caution.
	ModifyRoles,
	/// This permission allows the user to edit a workspace, but not delete it.
	/// Only the super admin of a workspace can delete it.
	EditWorkspace,
}

impl Permission {
	/// Returns a list of all permissions that can be granted on a resource.
	pub fn list_all_permissions() -> Vec<Self> {
		Self::iter().collect()
	}

	/// Returns the description of the permission, as per the documentation of
	/// the permission.
	pub fn description(&self) -> String {
		match self {
			Permission::Domain(permission) => permission.get_documentation(),
			Permission::DnsRecord(permission) => permission.get_documentation(),
			Permission::Deployment(permission) => permission.get_documentation(),
			Permission::ContainerRegistryRepository(permission) => permission.get_documentation(),
			Permission::Billing(permission) => permission.get_documentation(),
			Permission::ManagedURL(permission) => permission.get_documentation(),
			Permission::Runner(permission) => permission.get_documentation(),
			Permission::Database(permission) => permission.get_documentation(),
			Permission::StaticSite(permission) => permission.get_documentation(),
			Permission::Secret(permission) => permission.get_documentation(),
			Permission::Volume(permission) => permission.get_documentation(),
			Permission::ViewRoles => self.get_documentation(),
			Permission::ModifyRoles => self.get_documentation(),
			Permission::EditWorkspace => self.get_documentation(),
		}
		.expect("Documentation not found")
		.to_string()
	}
}

impl FromStr for Permission {
	type Err = strum::ParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let Some((permission_type, permission)) = s.split_once("::") else {
			return Err(strum::ParseError::VariantNotFound);
		};

		Ok(match permission_type {
			"domain" => Self::Domain(permission.parse()?),
			"dnsRecord" => Self::DnsRecord(permission.parse()?),
			"deployment" => Self::Deployment(permission.parse()?),
			"containerRegistryRepository" => Self::ContainerRegistryRepository(permission.parse()?),
			"billing" => Self::Billing(permission.parse()?),
			"managedURL" => Self::ManagedURL(permission.parse()?),
			"runner" => Self::Runner(permission.parse()?),
			"database" => Self::Database(permission.parse()?),
			"staticSite" => Self::StaticSite(permission.parse()?),
			"secret" => Self::Secret(permission.parse()?),
			"volume" => Self::Volume(permission.parse()?),
			"viewRoles" => Self::ViewRoles,
			"modifyRoles" => Self::ModifyRoles,
			"editWorkspace" => Self::EditWorkspace,
			_ => return Err(strum::ParseError::VariantNotFound),
		})
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<DB> sqlx::Type<DB> for Permission
where
	DB: sqlx::Database,
	String: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<String as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
		<String as sqlx::Type<DB>>::compatible(ty)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'q, DB> sqlx::Encode<'q, DB> for Permission
where
	DB: sqlx::Database,
	String: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
		<String as sqlx::Encode<'q, DB>>::encode(self.to_string(), buf)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'q, DB> sqlx::Decode<'q, DB> for Permission
where
	DB: sqlx::Database,
	String: sqlx::Decode<'q, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'q>,
	) -> Result<Self, sqlx::error::BoxDynError> {
		let permission = <String as sqlx::Decode<'q, DB>>::decode(value)?;
		Ok(FromStr::from_str(&permission)?)
	}
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

/// Represents the type of permission that is granted on a set of Resource IDs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, EnumDiscriminants)]
#[serde(
	rename_all = "camelCase",
	tag = "permissionType",
	content = "resources"
)]
#[strum_discriminants(
	name(ResourcePermissionTypeDiscriminant),
	derive(strum::Display),
	strum(serialize_all = "snake_case"),
	cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type)),
	cfg_attr(
		not(target_arch = "wasm32"),
		sqlx(type_name = "PERMISSION_TYPE", rename_all = "snake_case")
	)
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

impl ResourcePermissionType {
	/// Inserts a new resource ID into the current [`ResourcePermissionType`]
	/// instance based on the type of permission.
	pub fn insert(&mut self, resource_id: Uuid) {
		match self {
			Self::Include(resources) => {
				resources.insert(resource_id);
			}
			Self::Exclude(resources) => {
				resources.insert(resource_id);
			}
		}
	}
}
