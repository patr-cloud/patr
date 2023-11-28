use crate::{prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to update the domains DNS record
	VerifyDomainInWorkspace,
	POST "/workspace/:workspace_id/domain/:domain_id/verify" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The domain ID of the record
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
	},
	response = {
		/// Whether the domain is verified or not
		pub verified: bool,
	}
);
