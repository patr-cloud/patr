use crate::{
    prelude::*,
	utils::Uuid,
};

macros::declare_api_endpoint!(
	/// Route to delete a deployment
	DeleteDeployment,
	DELETE "/workspace/:workspace_id/infrastructure/deployment/:deployment_id",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
	query = {
		/// The workspace ID of the user
        pub workspace_id: Uuid,
		/// The deployment to be deleted
        pub deployment_id: Uuid
		/// Hard_delete will be considered only if it is byoc region
		/// if hard_delete not present, then take false as default
	    pub hard_delete: bool,
	}
);
