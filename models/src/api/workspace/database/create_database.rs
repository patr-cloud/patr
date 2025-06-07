use super::DatabaseEngine;
use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to create a new database
	/// Databases that are supported are MySQL, Postgress, MongoDB and Redis
	CreateDatabase,
	POST "/workspace/:workspace_id/infrastructure/database" {
		/// The ID of the workspace to get the info of
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::Database(DatabasePermission::Create),
		}
	},
	request = {
		/// The name of the database
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// The database engine (MySQL, MongoDB, Postgres, Redis)
		#[preprocess(none)]
		pub engine: DatabaseEngine,
		/// The database base plan ID (CPU, Memory, Volume)
		#[preprocess(none)]
		pub database_plan_id: Uuid,
		/// The region to deploy the database on
		#[preprocess(none)]
		pub region: Uuid,
		/// The database version to use
		#[preprocess(trim, lowercase)]
		pub version: String,
		/// The number of database instances to run following a master-slave architecture
		// #[preprocess(type = "u16")]
		pub num_node: u16
	},
	response = {
		/// The ID of the created database
		#[serde(flatten)]
		pub id: WithId<()>,
	}
);
