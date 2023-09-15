use crate::{
    prelude::*,
	utils::Uuid,
}; 

macros::declare_api_endpoint!(
    /// Route to revert a static site to an older version
    /// This route will revert the static site to an older release
    /// and will update the index.html file
    RevertStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site//:static_site_id/upload/:upload_id/revert",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The static site to revert
        pub static_site_id: Uuid,
        /// The upload_id to revert back to
        pub upload_id: Uuid,
    }
);