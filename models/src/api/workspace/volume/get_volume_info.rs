use super::DeploymentVolume;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to create a new volume
	GetVolumeInfo,
	GET "/volume/{volume_id}" {
		/// The volume ID of the volume to delete
		pub volume_id: Uuid,
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
			extract_resource_id: |req| req.path.volume_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Volume(VolumePermission::Delete),
		}
	},
	response = {
		/// The volume information
		#[serde(flatten)]
		pub volume: WithId<DeploymentVolume>,
	},
	audit_log = NoAuditLogger,
);
