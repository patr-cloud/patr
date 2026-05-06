use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete a runner
	DeleteRunner,
	DELETE "/runner/{runner_id}" {
		/// The ID of the runner
		pub runner_id: Uuid,
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
			extract_resource_id: |req| req.path.runner_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::Delete),
		}
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceDeleted,
		resource_type: ResourceType::Runner,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.runner_id),
	},
);
