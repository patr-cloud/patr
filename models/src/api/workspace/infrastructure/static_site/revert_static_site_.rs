use crate::prelude::*;

macros::declare_api_endpoint!(
    // Revert a static site to an older version
    RevertStaticSite,
    POST "/workspace/:workspace_id/infrastructure/static-site//:static_site_id/upload/:upload_id/revert",
    path = {
        pub workspace_id: Uuid,
        pub static_site_id: Uuid,
        pub upload_id: Uuid,
    }
);