use serde::{Deserialize, Serialize};

use crate::utils::Uuid;

mod get_recent_activity;
mod list_all_build_machine_type;

pub mod git_provider;
pub mod runner;

pub use self::{get_recent_activity::*, list_all_build_machine_type::*};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildMachineType {
	pub id: Uuid,
	pub cpu: i32,
	pub ram: i32,
	pub volume: i32,
}
