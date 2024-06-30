use serde::{Deserialize, Serialize};

mod create_volume;
mod delete_volume;
mod get_volume_info;
mod list_volumes;
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
