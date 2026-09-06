use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to create a new volume
	DeleteVolume,
	DELETE "/workspace/{workspace_id}/volume/{volume_id}" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The volume ID of the volume to delete
		pub volume_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.volume_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Volume(VolumePermission::Delete),
		}
	},
	client_type = [ApiToken, ServiceAccount, WebDashboard],
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceDeleted,
		resource_type: ResourceType::Volume,
		extract_resource_id: ResourceIdExtractor::FromRequest(|req| req.path.volume_id),
	},
);
