use crate::prelude::*;

macros::declare_api_endpoint!(
    // Create database
    CreateDatabase,
    POST "/workspace/:workspace_id/infrastructure/database",
    path ={
        pub workspace_id: Uuid
    },
    request = {
        pub name: String,
        pub db_name: String,
        pub version: String,
        pub engine: String,
        pub num_nodes: u16,
        pub database_plan: String,
        pub region: String,
    },
    response = {
        pub id: Uuid
    }
);