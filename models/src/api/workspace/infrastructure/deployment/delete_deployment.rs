use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
};

macros::declare_api_endpoint!(
	/// Route to delete a deployment
	DeleteDeployment,
	DELETE "/workspace/:workspace_id/infrastructure/deployment/:deployment_id"{
		/// The workspace ID of the user
        pub workspace_id: Uuid,
		/// The deployment to be deleted
        pub deployment_id: Uuid,
	},
	query = {
		/// Hard_delete will be considered only if it is byoc region
		/// if hard_delete not present, then take false as default
	    pub hard_delete: bool,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator { 
            extract_resource_id: |req| req.path.deployment_id
        }
	},
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
);
