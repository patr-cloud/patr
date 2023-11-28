use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to list all the users with the role
	ListUsersForRoles,
	GET "/workspace/:workspace_id/rbac/role/:role_id/users" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the role to get users for
		pub role_id: Uuid
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
	response = {
		/// The list of users with the role
		pub users: Vec<Uuid>
	}
);
