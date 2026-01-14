use std::str::FromStr;

use macros::RecursiveEnumIter;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumMessage, EnumString, IntoEnumIterator, VariantNames};

/// A list of all permissions that can be granted on a Database.
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
	Ord,
	Copy,
	Hash,
	Debug,
	Clone,
	Display,
	PartialEq,
	Serialize,
	Deserialize,
	PartialOrd,
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
	#[must_use]
	pub fn list_all() -> Vec<Self> {
		Self::iter().collect()
	}

	/// Returns the description of the permission, as per the documentation of
	/// the permission.
	///
	/// # Panics
	/// Panics if the permission does not have a documentation. This should
	/// not happen, as all permissions should have a documentation.
	#[must_use]
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
			Permission::ViewRoles | Permission::ModifyRoles | Permission::EditWorkspace => {
				self.get_documentation()
			}
		}
		.expect("Documentation not found")
		.to_string()
	}
}

impl FromStr for Permission {
	type Err = strum::ParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (permission_type, permission) = if let Some(split) = s.split_once("::") {
			split
		} else {
			(s, "")
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
use sqlx::{encode::IsNull, error::BoxDynError, prelude::*};

#[cfg(not(target_arch = "wasm32"))]
impl<DB> Type<DB> for Permission
where
	DB: sqlx::Database,
	String: Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		String::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
		String::compatible(ty)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'q, DB> Encode<'q, DB> for Permission
where
	DB: sqlx::Database,
	String: Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, sqlx::error::BoxDynError> {
		String::encode(self.to_string(), buf)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl<'q, DB> sqlx::Decode<'q, DB> for Permission
where
	DB: sqlx::Database,
	String: Decode<'q, DB>,
{
	fn decode(value: <DB as sqlx::Database>::ValueRef<'q>) -> Result<Self, BoxDynError> {
		let permission = <String as Decode<'q, DB>>::decode(value)?;
		Ok(FromStr::from_str(&permission)?)
	}
}
