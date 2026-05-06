use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete a secret
	DeleteSecret,
	DELETE "/secret/{secret_id}" {
		/// The ID of the secret to be deleted
		pub secret_id: Uuid,
	},
	workspaced = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.secret_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Secret(SecretPermission::Delete),
		}
	}
);
