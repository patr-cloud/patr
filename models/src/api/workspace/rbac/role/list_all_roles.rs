use super::Role;
use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to list all the roles
	ListAllRoles,
	GET "/workspaces/:workspace_id/rbac/role" {
		/// The ID of the workspace
		pub workspace_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	pagination = true,
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	response_headers = {
		/// The total number of items in the pagination
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of all roles that contains:
		///     name - The role name
		///     description - The role description
		pub roles: Vec<WithId<Role>>,
	}
);
