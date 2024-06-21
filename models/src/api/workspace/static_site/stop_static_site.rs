use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to stop a static site
	StopStaticSite,
	POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/stop" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The static site ID of static site to stop
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
			permission: Permission::StaticSite(StaticSitePermission::Stop),
		}
	}
);
