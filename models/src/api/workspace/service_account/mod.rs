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
use crate::prelude::*;

/// Represents a service account in a workspace.
///
/// A service account is a non-human identity used to authenticate runners and
/// other automated processes. It has a single token that can be regenerated,
/// and can be assigned roles within its workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct ServiceAccount {
	/// The name of the service account
	pub name: String,
	/// An optional description of what this service account is used for
	#[ts(type = "string | null")]
	pub description: Option<String>,
	/// The roles assigned to this service account
	#[search(skip)]
	pub roles: Vec<Uuid>,
}
