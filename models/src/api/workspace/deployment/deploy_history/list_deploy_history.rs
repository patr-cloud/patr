use super::DeploymentDeployHistory;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get list of deployment history for a deployment
	ListDeploymentDeployHistory,
	GET "/workspace/:workspace_id/deployment/:deployment_id/deploy-history" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the history for
		pub deployment_id: Uuid,
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
			permission: Permission::Deployment(DeploymentPermission::View),
		}
	},
	pagination = true,
	response_headers = {
		/// The total number of databases in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The deployment history containing:
		/// image_digest - The image digest of the deployment
		/// created - The timestamp of when the deployment was created
		pub deploys: Vec<DeploymentDeployHistory>
	}
);
