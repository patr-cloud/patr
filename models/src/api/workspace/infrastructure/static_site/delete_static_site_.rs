use crate::prelude::*;

macros::declare_api_endpoint!(
    // Delete a static site
    DeleteStaticSite,
    DELETE "/workspace/:workspace_id/infrastructure/static-site/:static_site_id",
    path = {
        pub workspace_id: Uuid,
        pub static_site_id: Uuid
    }
);