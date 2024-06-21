use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete the deployment history for a deployment
	DeleteDeploymentDeployHistory,
	DELETE "/workspace/:workspace_id/deployment/:deployment_id/deploy-history/:image_digest" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the history for
		pub deployment_id: Uuid,
		/// The image digest to delete
		pub image_digest: String,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id,
			permission: Permission::Deployment(DeploymentPermission::Edit)
		}
	}
);
