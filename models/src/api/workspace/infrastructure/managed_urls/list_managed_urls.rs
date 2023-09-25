use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
}; 
use super::ManagedUrl;

macros::declare_api_endpoint!(
    /// Route to list all managed URLs
    ListManagedUrls,
    GET "/workspace/:workspace_id/infrastructure/managed-url",
    request_headers = {
        /// Token used to authorize user
        pub access_token: BearerToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
    },
    response = {
        /// The list of all managed URLs present in the workspace containing:
        /// id - The managed URL ID
        /// sub_domain - The subdomain of the URL
        /// domain_id - The domain ID of the URL
        /// path - The URL path
        /// url_type - The type of URL (Deployment, Static Site, Proxy, Redirect)
        pub urls: Vec<ManagedUrl>,
    }
);