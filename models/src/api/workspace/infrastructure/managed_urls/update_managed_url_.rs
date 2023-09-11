use crate::prelude::*;

macros::declare_api_endpoint!(
    // Update a managed URL
    UpdateManagedUrl,
    POST "/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id",
    path = {
        pub workspace_id: Uuid,
        pub managed_url_id: Uuid,
    },
    request = {
        pub path: String,
        pub url_type: ManagedUrlType,
    },
);