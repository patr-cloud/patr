use crate::prelude::*;

macros::declare_api_endpoint!(
    // List all managed URLs
    ListManagedUrls,
    GET "/workspace/:workspace_id/infrastructure/managed-url",
    path = {
        pub workspace_id: Uuid,
    },
    response = {
        pub urls: Vec<ManagedUrl>,
    }
);