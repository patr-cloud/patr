use crate::prelude::*;

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
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.managed_url_id,
			permission: Permission::ManagedURL(ManagedURLPermission::Verify),
		}
	},
	response = {
		/// The status of the URL
		/// Is the URL configured or not
		pub configured: bool
	}
);
