use crate::prelude::*;

#[expect(clippy::trivially_copy_pass_by_ref)]
const fn is_false(force_restart: &bool) -> bool {
	!*force_restart
}

const fn default_force_restart() -> bool {
	false
}

macros::declare_api_endpoint!(
	/// Route to start a deployment
	StartDeployment,
	POST "/workspace/{workspace_id}/deployment/{deployment_id}/start" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID of the deployment to start
		pub deployment_id: Uuid,
	},
	query = {
		/// If true, the deployment will be force-restarted, even
		/// if it is already running
		#[serde(default = "default_force_restart", skip_serializing_if = "is_false")]
		pub force_restart: bool,
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
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Deployment(DeploymentPermission::Start),
		}
	}
);
