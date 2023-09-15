use crate::{
    prelude::*,
	utils::Uuid,
}; 

macros::declare_api_endpoint!(
    /// Route to stop a static site
    StopStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/stop",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The static site ID of static site to stop
        pub static_site_id: Uuid,
    }
);