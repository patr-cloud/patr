use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use crate::prelude::*;

/// All the modules that corresponds to Patr's in-build container registry
pub mod container_registry;
/// This module contains all the database models
pub mod database;
/// This module contains all the deployment models
pub mod deployment;
/// All the modules that corresponds to Patr Domains
pub mod domain;
/// This module contains all the managed URL models
pub mod managed_url;
/// This module contains all the models that corresponds to the RBAC of Patr
pub mod rbac;
/// This module contains all the models that corresponds to a runner of a Patr
/// workspace
pub mod runner;
/// This module contains all the models that corresponds to Patr secrets
pub mod secret;
/// This module contains all the static site models
pub mod static_site;
/// This module contains all the models that corresponds to a deployment volume
pub mod volume;

/// The endpoint to create a workspace
mod create_workspace;
/// The endpoint to delete a workspace
mod delete_workspace;
/// The endpoint to get the details of a workspace
mod get_workspace_info;
/// The endpoint to check if a workspace name is available
mod is_name_available;
/// The endpoint to update the details of a workspace
mod update_workspace_info;

pub use self::{
	create_workspace::*,
	delete_workspace::*,
	get_workspace_info::*,
	is_name_available::*,
	update_workspace_info::*,
};

/// The details of a workspace. A workspace contains all the resources that will
/// be created. A resource cannot exist outside of a workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
	/// The name of the workspace. This must be unique across Patr. This is used
	/// to identify it among the other workspaces in their account. In most
	/// cases, this would be their company name, for example.
	pub name: String,
	/// The userId of the user that is the super admin of this workspace. This
	/// user has the highest level of permissions in this workspace.
	pub super_admin_id: Uuid,
}

/// Logs corresponding to the actions performed on the workspace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceAuditLog {
	/// Date and time of the audit log
	pub date: OffsetDateTime,
	/// The IP address of the user who made the request
	pub ip_address: String,
	/// The workspace ID of the workspace the request was made in
	pub workspace_id: Uuid,
	/// The user ID of the user who made the request
	#[serde(skip_serializing_if = "Option::is_none")]
	pub user_id: Option<Uuid>,
	/// The login ID of the user who made the request
	#[serde(skip_serializing_if = "Option::is_none")]
	pub login_id: Option<Uuid>,
	/// The resource ID of the resource the request was made on
	pub resource_id: Uuid,
	/// The action that was performed on the resource
	pub action: String,
	/// The request ID of the request
	pub request_id: Uuid,
	/// The metadata of the request
	pub metadata: Value,
	/// Is it an action done by patr or not
	pub patr_action: bool,
	/// Was the request successful or not
	pub request_success: bool,
}
