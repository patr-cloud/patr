use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to revert a deployment to a previous deployment history
	RevertDeployment,
	POST "/deployment/{deployment_id}/deploy-history/{image_digest}/revert" {
		/// The deployment ID to get the history for
		pub deployment_id: Uuid,
		/// The image digest to delete
		pub image_digest: String,
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
			extract_resource_id: |req| req.path.deployment_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Deployment(DeploymentPermission::Edit)
		}
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Deployment,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.deployment_id),
	},
);
