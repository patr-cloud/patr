use crate::{prelude::*, utils::BearerToken};

use super::AddRegionToWorkspaceData;

macros::declare_api_endpoint!(
	/// Route to add region to a workspace
	AddRegionToWorkspace,
	POST "/workspace/:workspace_id/region" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
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
		/// Name of the region
		pub name: String,
		/// The region data
		pub data: AddRegionToWorkspaceData,
	},
	response = {
		/// The ID of the created region
		#[serde(flatten)]
		pub region_id: WithId<()>
	}
);
