use super::ManagedUrlType;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to update a managed URL configurations
	UpdateManagedURL,
	POST "/workspace/:workspace_id/infrastructure/managed-url/:managed_url_id" {
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
			permission: Permission::ManagedURL(ManagedURLPermission::Edit),
		}
	},
	request = {
		/// The new path of the updated URL
		#[preprocess(trim, lowercase)]
		pub path: String,
		/// The new type of the updated URL which can be
		/// Deployment, Static Site, Proxy or Redirect
		#[preprocess(none)]
		pub url_type: ManagedUrlType,
	},
);
