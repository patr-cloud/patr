use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to stop a deployment
	StopDeployment,
	POST "/workspace/{workspace_id}/deployment/{deployment_id}/stop" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID of the deployment to stop
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
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Deployment(DeploymentPermission::Stop),
		}
	},
	client_type = [ApiToken, ServiceAccount, WebDashboard],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Deployment,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.deployment_id),
	},
);
