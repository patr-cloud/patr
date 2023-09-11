use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get a static site info
    GetStaticSiteInfo,
    GET "/workspace/:workspace_id/infrastructure/static-site/:static_site_id",
    path = {
        pub workspace_id: Uuid,
        pub static_site_id: Uuid
    },
    response = {
        pub static_site: StaticSite,
        pub static_site_details: StaticSiteDetails
    }
);