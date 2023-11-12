use crate::{
	prelude::*,
	utils::BearerToken
};

macros::declare_api_endpoint!(
	/// Route to delete a database
	DeleteDatabase,
	DELETE "/workspace/:workspace_id/infrastructure/database/:database_id" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The ID of the database to be deleted
		pub database_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator { 
			extract_resource_id: |req| req.path.database_id 
		}
	}
);