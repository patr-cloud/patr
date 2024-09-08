use serde::{Deserialize, Serialize};

/// The endpoint to create a volume
mod create_volume;
/// The endpoint to delete a volume
mod delete_volume;
/// The endpoint to get the details of a volume
mod get_volume_info;
/// The endpoint to list all the volumes in a workspace
mod list_volumes;
/// The endpoint to update the details of a volume
mod update_volume;

pub use self::{
	create_volume::*,
	delete_volume::*,
	get_volume_info::*,
	list_volumes::*,
	update_volume::*,
};
use crate::prelude::*;

/// Deployment volume detail
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DeploymentVolume {
	/// The path of the volume attached
	pub name: String,
	/// The size of the volume
	pub size: u64,
	/// The deployment that the volume is attached to, if any
	pub deployment_id: Option<Uuid>,
}
