use super::DeploymentMachineType;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all machine types for deployment
	ListAllDeploymentMachineType,
	GET "/workspace/:workspace_id/deployment/machine-type" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	response = {
		/// The list of machine types available for deployment containing:
		/// cpu_count - The number of CPUs
		/// memory_count - The amount of RAM
		pub machine_types: Vec<WithId<DeploymentMachineType>>
	}
);
