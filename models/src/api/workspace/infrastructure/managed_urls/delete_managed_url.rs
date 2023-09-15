use crate::{
    prelude::*,
	utils::Uuid,
}; 

macros::declare_api_endpoint!(
    /// Route to delete a managed URL
    DeleteManagedUrl,
    DELETE "/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The manged URL ID to be deleted
        pub managed_url_id: Uuid,
    }
);