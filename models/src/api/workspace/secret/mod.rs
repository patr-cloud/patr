use serde::{Deserialize, Serialize};
use crate::prelude::*;

mod create_secret;
mod delete_secret;
mod list_secrets_for_workspace;
mod update_secret;

pub use self::{
	create_secret::*,
	delete_secret::*,
	list_secrets_for_workspace::*,
	update_secret::*,
};

/// Patr secrets which only contains the secret name and not the
/// secret value. This is to ensure that Patr does not have
/// access to any user sensitive information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
	/// The name of the secret
	pub name: String,
	/// The deployment the secret is attached to
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deployment_id: Option<Uuid>,
}
