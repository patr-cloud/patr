use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
}; 

macros::declare_api_endpoint!(
    /// Route to update a static site
    UpdateStaticSite,
    PATCH "/workspace/:workspace_id/infrastructure/static-site/:static_site_id" {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The static site ID of static site to update
        pub static_site_id: Uuid,
    },
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator { 
            extract_resource_id: |req| req.path.static_site_id
        }
	},
    request = {
        /// The updated static site name
        pub name: String,
    }
);