use super::DeploymentMachineType;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all machine types for deployment
	ListAllDeploymentMachineType,
	GET "/deployment/machine-type",
	workspaced = true,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	response = {
		/// The list of machine types available for deployment containing:
		/// cpu_count - The number of CPUs
		/// memory_count - The amount of RAM
		pub machine_types: Vec<WithId<DeploymentMachineType>>
	},
	audit_log = NoAuditLogger,
);
