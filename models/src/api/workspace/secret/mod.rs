use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::prelude::*;

/// The endpoint to create a secret in the workspace
mod create_secret;
/// The endpoint to delete a secret in the workspace
mod delete_secret;
/// The endpoint to get the information of a secret in the workspace
mod get_secret_info;
/// The endpoint to list all the secrets in the workspace
mod list_secrets_for_workspace;
/// The endpoint to update a secret in the workspace
mod update_secret;

pub use self::{
	create_secret::*,
	delete_secret::*,
	get_secret_info::*,
	list_secrets_for_workspace::*,
	update_secret::*,
};

/// Patr secrets which only contains the secret name and not the
/// secret value. This is to ensure that Patr does not have
/// access to any user sensitive information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
	/// The name of the secret
	pub name: String,
	/// The deployment the secret is attached to
	#[serde(skip_serializing_if = "Option::is_none")]
	#[search(ty = "resource", resource = "Deployment")]
	pub deployment_id: Option<Uuid>,
	/// The time the secret was created
	#[ts(type = "Date")]
	pub created: OffsetDateTime,
	/// The time the secret was last updated
	#[ts(type = "Date")]
	pub last_updated: OffsetDateTime,
}
