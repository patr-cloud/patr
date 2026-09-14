use serde_json::Value;

use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route for a runner to read the value of a secret in a workspace
	GetSecretForRunner,
	GET "/workspace/{workspace_id}/secret/{secret_id}/value" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the secret to read
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
			permission: Permission::Runner(RunnerPermission::Execute),
		}
	},
	response = {
		/// OpenBao's KV v2 read response, passed through as-is. The value is at
		/// `data.data.value`.
		pub secret: Value,
	},
	audit_log = NoAuditLogger,
);
