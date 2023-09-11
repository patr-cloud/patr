use crate::prelude::*;

macros::declare_api_endpoint!(
    // Upload to a static site
    UploadStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/upload",
    path = {
        pub workspace_id: Uuid,
        pub static_site_id: Uuid
    },
    request = {
        pub file: String,
        pub message: String
    },
    response = {
        pub upload_id: Uuid
    }
);