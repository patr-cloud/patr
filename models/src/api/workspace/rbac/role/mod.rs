/// The endpoint to create a new role in the workspace
mod create_new_role;
/// The endpoint to delete a role in the workspace
mod delete_role;
/// The endpoint to get the details of a role in the workspace
mod get_role_info;
/// The endpoint to list all the roles in the workspace
mod list_all_roles;
/// The endpoint to list every binding of a role in the workspace
mod list_role_bindings;
/// The endpoint to update the details of a role in the workspace
mod update_role;

use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;
use ts_rs::TS;

pub use self::{
	create_new_role::*,
	delete_role::*,
	get_role_info::*,
	list_all_roles::*,
	list_role_bindings::*,
	update_role::*,
};
use crate::{
	api::workspace::{rbac::user::WorkspaceUserInfo, service_account::ServiceAccount},
	prelude::*,
	utils::constants::{RESOURCE_NAME_REGEX, ROLE_DESCRIPTION_REGEX},
};

/// The role metadata
#[::preprocess::sync]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct Role {
	/// The name of the role
	#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
	pub name: String,
	/// The description of the role
	#[preprocess(trim, regex = ROLE_DESCRIPTION_REGEX)]
	#[serde(default, skip_serializing_if = "str::is_empty")]
	pub description: String,
	/// Whether the role is a seeded default that cannot be edited or deleted
	#[search(skip)]
	#[serde(default)]
	pub is_immutable: bool,
}

/// Who holds a binding: a member of the workspace or a service account in it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, EnumDiscriminants, TS)]
#[strum_discriminants(
	name(WorkspaceActorDiscriminant),
	cfg_attr(
		not(target_arch = "wasm32"),
		derive(sqlx::Type),
		sqlx(type_name = "WORKSPACE_ACTOR_TYPE", rename_all = "snake_case"),
	),
	doc = "Workspace actor types"
)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkspaceActor {
	/// A member of the workspace
	User(WorkspaceUserInfo),
	/// A service account in the workspace
	ServiceAccount(ServiceAccount),
}

/// One binding of a role: who holds it, and the resource it applies at.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct RoleBinding {
	/// Who holds the role. The ID is the user's for a member, and the service
	/// account's for a service account.
	pub actor: WithId<WorkspaceActor>,
	/// The resource the role applies at
	pub resource_id: Uuid,
}
