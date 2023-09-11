use crate::prelude::*;

macros::declare_api_endpoint!(
    // Delete a manged URL
    DeleteManagedUrl,
    DELETE "/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id",
    path = {
        pub workspace_id: Uuid,
        pub managed_url_id: Uuid,
    }
);