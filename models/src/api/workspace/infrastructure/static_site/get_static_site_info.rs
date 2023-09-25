use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
}; 
use super::StaticSiteDetails;

macros::declare_api_endpoint!(
    /// Route to get information of a static site
    /// This route will return the metadata of the static site along with details like metrics
    GetStaticSiteInfo,
    GET "/workspace/:workspace_id/infrastructure/static-site/:static_site_id",
    request_headers = {
        /// Token used to authorize user
        pub access_token: BearerToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The static site ID to get the information of
        pub static_site_id: Uuid
    },
    response = {
        /// The static site details which contains:
        /// ID - The ID of the static site
        /// name - The name of the static site
        /// status - The status of the static site 
        ///         (Created, Pushed, Deploying, Running, Stopped, Errored,Deleted)
        /// current_live_upload - The index.html that is currently live
        pub static_site: StaticSite,
        /// The static site details like metrics, etc
        pub static_site_details: StaticSiteDetails
    }
);