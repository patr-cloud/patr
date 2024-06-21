use super::Database;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get list of all database in a workspace
	ListDatabase,
	GET "/workspace/:workspace_id/infrastructure/database" {
		/// The workspace ID of the user
		pub workspace_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id:  |req| req.path.workspace_id
		}
	},
	pagination = true,
	response_headers = {
		/// The total number of databases in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// List of databases in the current workspace containing:
		/// id - The database ID
		/// name - The database name
		/// engine - The database engine (MySQL, Postgres, MongoDB, Redis)
		 /// version - The database version
		/// num_node - The number of database instances
		/// database_plan_id - The database plan which corresponds to CPU, RAM and Disk
		/// region - The region the database is deployed on
		/// status - The current status of the database
		/// public_connection - The connection configuration of the database which contains:
		///                     host - The database host IP
		///                     port - The connection port
		///                     username - The amin username
		///                     password - The admin password
		pub database: Vec<WithId<Database>>
	}
);
