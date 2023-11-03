use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
}; 
use super::ManagedUrl;

macros::declare_api_endpoint!(
    /// Route to list all managed URLs
    ListManagedURL,
    GET "/workspace/:workspace_id/infrastructure/managed-url" {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
    },
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id,
		}
	},
    response = {
        /// The list of all managed URLs present in the workspace containing:
        /// sub_domain - The subdomain of the URL
        /// domain_id - The domain ID of the URL
        /// path - The URL path
        /// url_type - The type of URL (Deployment, Static Site, Proxy, Redirect)
        pub urls: Vec<WithId<ManagedUrl>>,
    }
);