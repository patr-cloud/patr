use serde::{Deserialize, Serialize};

use crate::utils::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
	pub id: Uuid,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deployment_id: Option<Uuid>,
}
