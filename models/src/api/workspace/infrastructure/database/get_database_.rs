use crate::prelude::*;
use super::ManagedDatabase;

macros::declare_api_endpoint!(
    // Get database info
    GetDatabase,
    GET "/workspace/:workspace_id/infrastructure/database/:database_id",
    path = {
        pub workspace_id: Uuid,
        pub database_id: Uuid
    },
    response = {
        pub database: Database,
    }
);