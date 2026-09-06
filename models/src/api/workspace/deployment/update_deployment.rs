use super::DeploymentRunningDetails;
use crate::{
	prelude::*,
	utils::constants::{DEPLOYMENT_IMAGE_TAG_REGEX, RESOURCE_NAME_REGEX},
};

macros::declare_api_endpoint!(
	/// Route to update a deployment
	UpdateDeployment,
	PATCH "/workspace/{workspace_id}/deployment/{deployment_id}" {
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
			permission: Permission::Deployment(DeploymentPermission::Edit),
		}
	},
	request = {
		/// The name of the deployment
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// The image tag to use
		#[preprocess(trim, lowercase, regex = DEPLOYMENT_IMAGE_TAG_REGEX)]
		pub image_tag: String,
		/// The runner to use to run the deployment
		#[preprocess(none)]
		pub runner: Uuid,
		/// The machine type the deployment pod will run on
		#[preprocess(none)]
		pub machine_type: Uuid,
		/// The details of the deployment which contains information related to configuration
		#[preprocess(none)]
		#[serde(flatten)]
		pub running_details: DeploymentRunningDetails,
	},
	client_type = [ApiToken, ServiceAccount, WebDashboard],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Deployment,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.deployment_id),
	},
);
