use crate::{
    prelude::*,
	utils::Uuid,
}; 
use super::StaticSiteDetails;

macros::declare_api_endpoint!(
    /// Route to list all static site in a workspace
    ListStaticSites,
    GET "/workspace/:workspace_id/infrastructure/static-site",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
    },
    response = {
        /// The list of static site in the workspace
        /// The list contains:
        /// ID - The ID of the static site
        /// name - The name of the static site
        /// status - The status of the static site 
        ///         (Created, Pushed, Deploying, Running, Stopped, Errored,Deleted)
        /// current_live_upload - The index.html that is currently live
        pub static_sites: Vec<StaticSite>
    }
);