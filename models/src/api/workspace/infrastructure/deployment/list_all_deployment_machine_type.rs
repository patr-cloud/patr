use crate::prelude::*;
use super::DeploymentMachineType;

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