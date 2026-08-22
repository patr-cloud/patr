use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to update a secret
	UpdateSecret,
	PATCH "/workspace/{workspace_id}/secret/{secret_id}" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the secret to be deleted
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
			permission: Permission::Secret(SecretPermission::Edit),
		}
	},
	request = {
		/// The updated name of the secret
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// The updated value of the secret. When omitted, the existing value is
		/// kept; when present, the secret is rotated to the new value.
		#[preprocess(none)]
		pub value: Option<String>,
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceUpdated,
		resource_type: ResourceType::Secret,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.secret_id),
	},
);
