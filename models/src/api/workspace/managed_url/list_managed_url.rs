use super::*;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to list all managed URLs
	ListManagedURL,
	GET "/workspace/:workspace_id/infrastructure/managed-url" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id
		}
	},
	query = {
		/// The order to sort the list of managed URLs
		pub order: Option<ListOrder>,
		/// The field to order the list of managed URLs by
		pub order_by: Option<ListOrderBy>,
		/// Search by a specific query
		pub filter: Option<String>,
	},
	pagination = true,
	response_headers = {
		/// The total number of Managed URLs in the requested workspace
		pub total_count: TotalCountHeader,
	},
	response = {
		/// The list of all managed URLs present in the workspace containing:
		/// sub_domain - The subdomain of the URL
		/// domain_id - The domain ID of the URL
		/// path - The URL path
		/// url_type - The type of URL (Deployment, Static Site, Proxy, Redirect)
		pub urls: Vec<WithId<ManagedUrl>>,
	}
);
