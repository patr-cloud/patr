use crate::{
	prelude::*,
	utils::{Uuid, BearerToken},
}; 

macros::declare_api_endpoint!(
	/// Route to revert a deployment to an older digest
	RevertDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/deploy-history/:digest/revert" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to revert
		pub deployment_id: Uuid,
		/// The deployment digest to revert to
		pub digest: String
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
