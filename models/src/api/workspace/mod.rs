use serde::{Deserialize, Serialize};

use crate::prelude::*;

// All the routes that corresponds to Patr's in-build container registry
// pub mod container_registry;
// pub mod domain;
// pub mod infrastructure;
// pub mod rbac;
// pub mod region;
// pub mod secret;

mod create_workspace;
mod delete_workspace;
mod get_workspace_info;
mod is_name_available;
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
