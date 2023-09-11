use crate::prelude::*;

macros::declare_api_endpoint!(
    // Stop a static site
    StopStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/stop",
    path = {
        pub workspace_id: Uuid,
        pub static_site_id: Uuid
    }
);