use crate::{
	prelude::*,
	utils::{BearerToken, Uuid},
};

macros::declare_api_endpoint!(
	/// Route to start a deployment
	StartDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/start" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID of the deployment to start
		pub deployment_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id
		}
	}
);
