use crate::prelude::*;

macros::declare_api_endpoint!(
    // Create static site
    CreateStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site",
    path = {
        pub workspace_id: Uuid
    },
    request = {
        pub name: String,
        pub message: String,
        pub file: Option<String>,
        pub static_site_details: StaticSiteDetails,
    },
    response = {
        pub id: Uuid
    }
);