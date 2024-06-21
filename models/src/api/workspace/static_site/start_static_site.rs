use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to start a static site
	StartStaticSite,
	POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/start" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The static site ID of static site to start
		pub static_site_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.static_site_id,
			permission: Permission::StaticSite(StaticSitePermission::Start),
		}
	}
);
