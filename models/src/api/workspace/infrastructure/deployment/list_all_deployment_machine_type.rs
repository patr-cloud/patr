use crate::prelude::*;
use serde::{Deserialize, Serialize};

/// Information of all the different deployment plans currently supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentMachineType {
    /// The number of CPU nodes
	pub cpu_count: i16,
    /// The number of memory nodes
	pub memory_count: i32,
}

macros::declare_api_endpoint!(
    /// Route to list all machine types for deployment
    ListAllDeploymentMachineType,
    GET "/workspace/infrastructure/machine-type",
    response = {
        /// The list of machine types available for deployment containing:
        /// cpu_count - The number of CPUs
        /// memory_count - The amount of RAM
        pub machine_types: Vec<WithId<DeploymentMachineType>>
    }
);