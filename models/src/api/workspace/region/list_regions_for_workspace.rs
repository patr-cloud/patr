use crate::{prelude::*, utils::BearerToken};
use super::Region;

macros::declare_api_endpoint!(
	/// Route to list all the regions of a workspace
	ListRegionsForWorkspace,
	GET "/workspace/:workspace_id/region" {
		/// The ID of the workspace
		pub workspace_id: Uuid
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
	response = {
		/// The region information containing:
		///     name - The name of the region
		///     cloud_provider - The cloud provider the region is of
		///     status - The status of the region
		///     r#type - The region type
		pub regions: Vec<WithId<Region>>,
	}
);
