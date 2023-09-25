use crate::{
    prelude::*,
	utils::{Uuid,BearerToken},
}; 

macros::declare_api_endpoint!(
    /// Route to verify a managed URL configuration
    VerifyManagedUrlConfiguration,
    POST "/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id/verify-configuration",
    request_headers = {
        /// Token used to authorize user
        pub access_token: BearerToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
    },
    response = {
        /// The status of the URL
        /// Is the URL configured of not
        pub configured: bool
    }
);