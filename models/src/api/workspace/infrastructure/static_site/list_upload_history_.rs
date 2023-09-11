use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get all upload history of a static site
    ListStaticSiteUploadHistory,
    GET "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/upload",
    path = {
        pub workspace_id: Uuid,
        pub static_site_id: Uuid,
    },
    response = {
        pub uploads: Vec<StaticSiteUploadHistory>
    }
);