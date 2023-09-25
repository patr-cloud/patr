use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
	models::workspace::infrastructure::managed_urls::ManagedUrl,
}; 

macros::declare_api_endpoint!(
    /// Route to get all linked URLs for a deployment
    ListLinkedURLs,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/managed-urls",
    request_headers = {
        /// Token used to authorize user
        pub access_token: BearerToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The deployment ID to get the history for
        pub deployment_id: Uuid
    },
    response = {
        /// The list of linked managed URL to a deployment containing:
        /// id - The managed URL ID
        /// sub_domain - The subdomain of the URL
        /// domain_id - The domain ID of the URL
        /// path - The URL path
        /// url_type - The type of URL (Deployment, Static Site, Proxy, Redirect)
        pub urls: Vec<ManagedUrl>
    }
);
