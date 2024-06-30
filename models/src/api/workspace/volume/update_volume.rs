use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to create a new volume
	UpdateVolume,
	PATCH "/workspace/:workspace_id/volume/:volume_id" {
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
			permission: Permission::Volume(VolumePermission::Edit),
		}
	},
	request = {
		/// The name of the volume
		#[preprocess(optional(trim, regex = RESOURCE_NAME_REGEX))]
		pub name: Option<String>,
		/// The size of the volume
		#[preprocess(optional(range(min = 1)))]
		pub size: Option<u64>,
	}
);
