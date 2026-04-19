use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to update a workspace's info based on the ID
	UpdateWorkspaceInfo,
	PATCH "/workspace/{workspace_id}" {
		/// The ID of the workspace to update
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::EditWorkspace,
		}
	},
	request = {
		/// The new name of the workspace
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
	},
	client_type = [ApiToken, ServiceAccount, WebDashboard],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Workspace,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.workspace_id),
	},
);
