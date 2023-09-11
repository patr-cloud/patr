use crate::prelude::*;

macros::declare_api_endpoint!(
    // Delete database
    DeleteDatabase,
    DELETE "/workspace/:workspace_id/infrastructure/database/:database_id",
    path = {
        pub workspace_id: Uuid,
        pub database_id: Uuid
    }
);