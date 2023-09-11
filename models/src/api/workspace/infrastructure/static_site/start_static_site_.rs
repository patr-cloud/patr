use crate::prelude::*;

macros::declare_api_endpoint!(
    // Start a static site
    StartStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/start",
    path = {
        pub workspace_id: Uuid,
        pub static_site_id: Uuid,
    }
);