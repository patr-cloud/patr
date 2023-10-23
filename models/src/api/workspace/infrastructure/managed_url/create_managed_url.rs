use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
}; 
use super::ManagedUrlType;

macros::declare_api_endpoint!(
    /// Route to create a new managed URL
    CreateManagedUrl,
    POST "/workspace/:workspace_id/infrastructure/managed-url" {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
    },
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id,
		}
	},
    request = {
        /// The sub domain of the URL
        pub sub_domain: String,
        /// The domain ID
        pub domain_id: Uuid,
        /// The path of the URL
        pub path: String,
        /// The URL type (Deployment, Static Site, Proxy or Redirect)
        pub url_type: ManagedUrlType,
    },
    response = {
        /// The new managed URL ID
        pub id: Uuid,
    }
);