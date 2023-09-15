use crate::{
    prelude::*,
	utils::Uuid,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentMachineType {
	pub id: Uuid,
	pub cpu_count: i16,
	pub memory_count: i32,
}

macros::declare_api_endpoint!(
    /// Route to list all machine types for deployment
    ListAllDeploymentMachineTypes,
    GET "/workspace/:workspace_id/infrastructure/machine-type",
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid
    },
    response = {
        /// The list of machine types available for deployment containing:
        /// id - The machine type ID
        /// cpu_count - The number of CPUs
        /// memory_count - The amount of RAM
        pub machine_types: Vec<DeploymentMachineType>
    }
);