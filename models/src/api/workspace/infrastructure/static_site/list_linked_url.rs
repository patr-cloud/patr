use crate::{api::workspace::infrastructure::managed_url::ManagedUrl, prelude::*, utils::Uuid};

macros::declare_api_endpoint!(
	/// Route to get all the linked URLs with a static site
	ListLinkedURL,
	GET "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/managed-urls" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The static site ID to retrieve the linked URLs for
		pub static_site_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.static_site_id
		}
	},
	pagination = true,
	response_headers = {
		/// The total number of databases in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of linked URLs linked to the static site which contain:
		/// sub_domain - The subdomain of the URL
		/// domain_id - The domain ID of the URL
		/// path - The URL path
		/// url_type - The type of URL (Deployment, Static Site, Proxy, Redirect)
		pub urls: Vec<WithId<ManagedUrl>>
	}
);
