/// The endpoint to create a new role in the workspace
mod create_new_role;
/// The endpoint to delete a role in the workspace
mod delete_role;
/// The endpoint to get the details of a role in the workspace
mod get_role_info;
/// The endpoint to list all the roles in the workspace
mod list_all_roles;
/// The endpoint to list all the users for a role in the workspace
mod list_users_for_role;
/// The endpoint to update the details of a role in the workspace
mod update_role;

use serde::{Deserialize, Serialize};

pub use self::{
	create_new_role::*,
	delete_role::*,
	get_role_info::*,
	list_all_roles::*,
	list_users_for_role::*,
	update_role::*,
};

/// The role metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Role {
	/// The name of the role
	pub name: String,
	/// The description of the role
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub description: String,
}
