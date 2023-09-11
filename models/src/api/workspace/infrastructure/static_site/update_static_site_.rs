use crate::prelude::*;

macros::declare_api_endpoint!(
    // Update a static site
    UpdateStaticSite,
    PATCH "/workspace/:workspace_id/infrastructure/static-site/:static_site_id",
    path = {
        pub workspace_id: Uuid,
        pub static_site_id: Uuid,
    },
    request = {
        pub name: String,
    }
);