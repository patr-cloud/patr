use crate::prelude::*;

macros::declare_api_endpoint!(
    // List all static site
    ListStaticSites,
    GET "/workspace/:workspace_id/infrastructure/static-site",
    path = {
        pub workspace_id: Uuid,
    },
    response = {
        pub static_sites: Vec<StaticSite>
    }
);