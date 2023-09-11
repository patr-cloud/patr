use crate::prelude::*;

macros::declare_api_endpoint!(
    // Verify a managed URL configuration
    VerifyManagedUrlConfiguration,
    POST "/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id/verify-configuration",
    path = {
        pub workspace_id: Uuid,
    },
    response = {
        pub configured: bool
    }
);