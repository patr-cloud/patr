use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
};
use super::StaticSite; 

macros::declare_api_endpoint!(
    /// Route to list all static site in a workspace
    ListStaticSite,
    GET "/workspace/:workspace_id/infrastructure/static-site" {
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
    response = {
        /// The list of static site in the workspace
        /// The list contains:
        /// name - The name of the static site
        /// status - The status of the static site 
        ///         (Created, Pushed, Deploying, Running, Stopped, Errored,Deleted)
        /// current_live_upload - The index.html that is currently live
        pub static_sites: Vec<WithId<StaticSite>>
    }
);