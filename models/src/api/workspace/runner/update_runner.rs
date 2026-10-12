use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to update a runner
	UpdateRunner,
	PATCH "/workspace/{workspace_id}/runner/{runner_id}" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the runner to update
		pub runner_id: Uuid,
	},
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
			permission: Permission::Runner(RunnerPermission::Edit),
		}
	},
	request = {
		/// The updated name of the runner
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Runner,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.runner_id),
	},
);
