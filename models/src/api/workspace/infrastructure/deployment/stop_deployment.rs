use crate::{
	prelude::*,
	utils::{Uuid, BearerToken},
}; 

macros::declare_api_endpoint!(
	/// Route to stop a deployment
	StopDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/stop" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID of the deployment to stop
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
