use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to delete a region
	DeleteRegion,
	DELETE "/workspace/:workspace_id/region/:region_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The region ID 
		pub region_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// Whether or not to hard delete a region
		/// If hard_delete not present, then take false as default
		/// NOTE: hard_delete should be shown in frontend iff RegionStatus::Active,
		///       else the region won't get deleted due to kubeconfig error
		pub hard_delete: bool,
	}
);
