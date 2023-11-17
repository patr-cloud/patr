use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to delete domain in a workspace
	DeleteDomainInWorkspace,
	DELETE "/workspace/:workspace_id/domain/:domain_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The domain ID of the workspace
		pub domain_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.domain_id
		}
	}
);
