use super::DatabasePlan;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get database information
	ListAllDatabaseMachineType,
	GET "/workspace/infrastructure/database/plan",
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	response = {
		/// List of database plans containing:
		/// cpu_count: The number of CPU nodes
		/// memory_count: The number of memory nodes
		/// volume: The size of the volume
		pub plans: Vec<WithId<DatabasePlan>>
	}
);
