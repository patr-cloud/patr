use crate::{prelude::*, utils::BearerToken, permission::WorkspacePermission};

macros::declare_api_endpoint!(
	/// Route to get current permissions
	GetCurrnetPermissions,
	GET "/workspace/:workspace_id/rbac/current-permissions" {
		/// The ID of the workspace
		pub workspace_id: Uuid
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
		/// The permissions
		#[serde(flatten)]
		pub permissions: WorkspacePermission
	}
);
