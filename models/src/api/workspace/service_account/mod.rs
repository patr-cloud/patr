/// The endpoint to create a service account in a workspace
mod create_service_account;
/// The endpoint to delete a service account
mod delete_service_account;
/// The endpoint to get the details of a service account
mod get_service_account_info;
/// The endpoint to list all service accounts in a workspace
mod list_service_accounts;
/// The endpoint to regenerate a service account's token
mod regenerate_service_account_token;
/// The endpoint to update a service account
mod update_service_account;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use self::{
	create_service_account::*,
	delete_service_account::*,
	get_service_account_info::*,
	list_service_accounts::*,
	regenerate_service_account_token::*,
	update_service_account::*,
};
use crate::{api::workspace::rbac::user::RoleBindingGrant, prelude::*};

/// Represents a service account in a workspace.
///
/// A service account is a non-human identity used to authenticate runners and
/// other automated processes. It has a single token that can be regenerated,
/// and is granted access through role bindings like any other actor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccount {
	/// The name of the service account
	pub name: String,
	/// An optional description of what this service account is used for
	#[ts(type = "string | null")]
	pub description: Option<String>,
	/// The role grants this service account holds — each a role and the
	/// resource it applies at.
	#[search(skip)]
	pub role_bindings: Vec<RoleBindingGrant>,
}
