use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get all linked URLs for a deployment
    ListLinkedURLs,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/managed-urls",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
    response = {
        pub urls: Vec<ManagedUrl>
    }
);
