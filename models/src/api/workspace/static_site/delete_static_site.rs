use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to delete a static site
	/// This route will permanently delete the static site including it's history
	/// and the current index.html file
	DeleteStaticSite,
	DELETE "/infrastructure/static-site/{static_site_id}" {
		/// The static site ID to be deleted
		pub static_site_id: Uuid
	},
	workspaced = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.static_site_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::StaticSite(StaticSitePermission::Delete),
		}
	}
);
