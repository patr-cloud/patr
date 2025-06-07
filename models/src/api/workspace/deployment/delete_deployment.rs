use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete a deployment
	DeleteDeployment,
	DELETE "/workspace/:workspace_id/deployment/:deployment_id"{
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment to be deleted
		pub deployment_id: Uuid,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id,
			permission: Permission::Deployment(DeploymentPermission::Delete),
		}
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
);
