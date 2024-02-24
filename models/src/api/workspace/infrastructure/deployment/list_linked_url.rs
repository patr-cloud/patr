use crate::{api::workspace::infrastructure::managed_url::ManagedUrl, prelude::*};

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
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id
		}
	},
	pagination = true,
	response_headers = {
		/// The total number of databases in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of linked managed URL to a deployment containing:
		/// sub_domain - The subdomain of the URL
		/// domain_id - The domain ID of the URL
		/// path - The URL path
		/// url_type - The type of URL (Deployment, Static Site, Proxy, Redirect)
		pub urls: Vec<WithId<ManagedUrl>>
	}
);
