use super::AddRegionToWorkspaceData;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to add region to a workspace
	AddRegionToWorkspace,
	POST "/workspace/:workspace_id/region" {
		/// The ID of the workspace
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
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// Name of the region
		#[preprocess(lowercase)]
		pub name: String,
		/// The region data
		#[preprocess(none)]
		pub data: AddRegionToWorkspaceData,
	},
	response = {
		/// The ID of the created region
		#[serde(flatten)]
		pub region_id: WithId<()>
	}
);
