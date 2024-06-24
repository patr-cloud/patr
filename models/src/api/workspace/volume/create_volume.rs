use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to create a new volume
	CreateVolume,
	POST "/workspace/:workspace_id/volume" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::Volume(VolumePermission::Create),
		}
	},
	request = {
		/// The name of the volume
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// The size of the volume
		#[preprocess(range(min = 1))]
		pub size: u16,
	},
	response = {
		/// The ID of the created volume
		#[serde(flatten)]
		pub id: WithId<()>,
	}
);
