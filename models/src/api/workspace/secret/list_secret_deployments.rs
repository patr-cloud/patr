use super::SecretDeployment;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list the deployments using a secret, and the variables they use it as
	ListSecretDeployments,
	GET "/workspace/{workspace_id}/secret/{secret_id}/deployment" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the secret to list the deployments of
		pub secret_id: Uuid,
	},
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
			permission: Permission::Secret(SecretPermission::View),
		}
	},
	listable_resource = SecretDeployment,
	response_headers = {
		/// The total number of deployments the caller can view using this secret
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The deployments using this secret that the caller can view
		pub deployments: Vec<WithId<SecretDeployment>>,
	},
	client_type = [ApiToken, ServiceAccount, WebLogin],
	audit_log = NoAuditLogger,
);
