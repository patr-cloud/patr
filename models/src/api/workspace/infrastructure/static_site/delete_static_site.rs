use crate::{
    prelude::*,
	utils::Uuid,
}; 

macros::declare_api_endpoint!(
    /// Route to delete a static site
    /// This route will permenantly delete the static site including it's history 
    /// and the current index.html file
    DeleteStaticSite,
    DELETE "/workspace/:workspace_id/infrastructure/static-site/:static_site_id",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The static site ID to be deleted
        pub static_site_id: Uuid
    }
);