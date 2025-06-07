use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to revert a static site to an older version
	/// This route will revert the static site to an older release
	/// and will update the index.html file
	RevertStaticSite,
	POST "/workspace/:workspace_id/infrastructure/static-site/:static_site_id/upload/:upload_id/revert" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The static site to revert
		pub static_site_id: Uuid,
		/// The upload_id to revert back to
		pub upload_id: Uuid,
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
			permission: Permission::StaticSite(StaticSitePermission::Edit),
		}
	}
);
