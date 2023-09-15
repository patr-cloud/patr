use crate::prelude::*;

macros::declare_api_endpoint!(
    /// Route to delete a database
    DeleteDatabase,
    DELETE "/workspace/:workspace_id/infrastructure/database/:database_id",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The ID of the database to be deleted
        pub database_id: Uuid
    }
);