use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The permission metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
	/// The name of the permission
	pub name: String,
	/// The description of the permission
	pub description: String,
}

macros::declare_api_endpoint!(
	/// Route to list all the permissions
	ListAllPermissions,
	GET "/workspace/:workspace_id/rbac/permission" {
		/// The ID of the workspace
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
			extract_workspace_id: |req| req.path.workspace_id
		}
	},
	response = {
		/// The list permissions that contains:
		/// - name - The name of the permission
		/// - description - The description of the permission
		pub permissions: Vec<WithId<Permission>>
	}
);
