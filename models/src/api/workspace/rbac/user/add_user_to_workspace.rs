use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to add user to a workspace
	AddUserToWorkspace,
	POST "/workspace/:workspace_id/rbac/user/:user_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The user ID of the user to add to the workspace
		pub user_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// The list of roles the user has after being
		/// added to the workspace
		pub roles: Vec<Uuid>,
	},
);
