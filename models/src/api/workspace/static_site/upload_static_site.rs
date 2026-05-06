use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to upload to a static site
	/// This route will upload a new index.html file which would go live
	UploadStaticSite,
	POST "/infrastructure/static-site/{static_site_id}/upload" {
		/// The static site ID of static site to upload index.html file
		pub static_site_id: Uuid,
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
			permission: Permission::StaticSite(StaticSitePermission::Upload),
		}
	},
	request = {
		/// The new index.html file
		#[preprocess(trim, lowercase)]
		pub file: String,
		/// The release note (eg: v1.0.0)
		#[preprocess(trim, lowercase)]
		pub message: String
	},
	response = {
		/// The upload ID of the new upload
		#[serde(flatten)]
		pub upload_id: OnlyId
	}
);
