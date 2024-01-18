use crate::{
	prelude::*,
	utils::{BearerToken, Uuid},
};

macros::declare_api_endpoint!(
	/// Route to verify a managed URL configuration
	VerifyManagedURLConfiguration,
	POST "/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id/verify-configuration" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The managed URL to be deleted
		pub managed_url_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.managed_url_id
		}
	},
	response = {
		/// The status of the URL
		/// Is the URL configured of not
		pub configured: bool
	}
);
