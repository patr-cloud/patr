mod create_new_role;
mod delete_role;
mod get_role_info;
mod list_all_roles;
mod list_users_for_role;
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
