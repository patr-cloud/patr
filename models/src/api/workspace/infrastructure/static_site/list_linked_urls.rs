use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
    models::workspace::infrastructure::managed_urls::ManagedUrl,
}; 

macros::declare_api_endpoint!(
    /// Route to get all the linked URLs with a static site
    ListLinkedURLs,
    GET "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/managed-urls",
    request_headers = {
        /// Token used to authorize user
        pub access_token: BearerToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The static site ID to retrieve the linked URLs for
        pub statis_site_id: Uuid
    },
    response = {
        /// The list of linked URLs linked to the static site which contain:
        /// id - The managed URL ID
        /// sub_domain - The subdomain of the URL
        /// domain_id - The domain ID of the URL
        /// path - The URL path
        /// url_type - The type of URL (Deployment, Static Site, Proxy, Redirect)
        pub urls: Vec<ManagedUrl>
    }
);