use crate::{
    prelude::*,
	utils::Uuid,
}; 

macros::declare_api_endpoint!(
    /// Route to upload to a static site
    /// This route will upload a new index.html file which would go live
    UploadStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/upload",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The static site ID of static site to upload index.html file
        pub static_site_id: Uuid,
    }
    request = {
        /// The new index.html file
        pub file: String,
        /// The release note (eg: v1.0.0)
        pub message: String
    },
    response = {
        /// The upload ID of the new upload
        pub upload_id: Uuid
    }
);