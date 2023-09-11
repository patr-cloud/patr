use crate::prelude::*;

macros::declare_api_endpoint!(
    // Create managed URL
    CreateNewManagedUrl,
    POST "/workspace/:workspace_id/infrastructure/managed-url",
    path = {
        pub workspace_id: Uuid,
    },
    request = {
        pub sub_domain: String,
        pub domain_id: Uuid,
        pub path: String,
        pub url_type: ManagedUrlType,
    },
    response = {
        pub id: Uuid,
    }
);