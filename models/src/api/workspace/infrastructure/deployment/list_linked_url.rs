use crate::{
	api::workspace::infrastructure::managed_url::ManagedUrl,
	prelude::*,
	utils::{BearerToken, Uuid},
};

macros::declare_api_endpoint!(
	/// Route to get all linked URLs for a deployment
	ListLinkedURL,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/managed-urls" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the history for
		pub deployment_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id
		}
	},
	pagination = true,
	response = {
		/// The list of linked managed URL to a deployment containing:
		/// sub_domain - The subdomain of the URL
		/// domain_id - The domain ID of the URL
		/// path - The URL path
		/// url_type - The type of URL (Deployment, Static Site, Proxy, Redirect)
		pub urls: Vec<WithId<ManagedUrl>>
	}
);
