use super::Database;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get database information
	GetDatabase,
	GET "/workspace/:workspace_id/infrastructure/database/:database_id" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The database ID to retrieve database information
		pub database_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.database_id,
			permission: Permission::Database(DatabasePermission::View)
		}
	},
	response = {
		/// The database information containing:
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
		pub database: WithId<Database>
	}
);
