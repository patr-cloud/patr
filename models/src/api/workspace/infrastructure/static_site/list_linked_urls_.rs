use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get all linked URLs with a static site
    ListLinkedURLs,
    GET "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/managed-urls",
    path = {
        pub workspace_id: Uuid,
        pub statis_site_id: Uuid
    },
    response = {
        pub urls: Vec<ManagedUrl>
    }
);