use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to add runner to a workspace
	AddRunnerToWorkspace,
	POST "/runner",
	workspaced = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::Create),
		}
	},
	request = {
		/// Name of the runner
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
	},
	response = {
		/// The ID of the created runner
		#[serde(flatten)]
		pub id: OnlyId,
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceCreated,
		resource_type: ResourceType::Runner,
		extract_resource_id: ResourceIdExtractor::FromResponse(|res| res.body.id.id),
	},
);
