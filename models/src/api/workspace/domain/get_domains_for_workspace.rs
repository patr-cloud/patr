use crate::{prelude::*, utils::BearerToken};
use super::WorkspaceDomain;

macros::declare_api_endpoint!(
	/// Route to get all the domains in a workspace
	GetDomainsForWorkspace,
	GET "/workspace/:workspace_id/domain" {
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
	response = {
		/// The list of domains containing:
		///     domain - The domain metadata
		///     is_verified - whether the domain is verified or not
		///     nameserver_type - The type of the nameserver
		pub domains: Vec<WithId<WorkspaceDomain>>,
	}
);
