use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to update a secret
	UpdateSecret,
	PATCH "/workspace/:workspace_id/secret/:secret_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The ID of the secret to be deleted
		pub secret_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.secret_id
		}
	},
	request = {
		/// The updated name of the secret
		#[preprocess(trim, uppercase)]		
		pub name: Option<String>,
		/// The updated value of the secret
		#[preprocess(trim)]
		pub value: Option<String>,
	}
);
